#!/usr/bin/env python3
"""Strategic Compact Suggester (Python port).

Cross-platform (Windows, macOS, Linux). Runs on PreToolUse to suggest manual
`/compact` at logical intervals rather than relying on arbitrary auto-compaction.

Why manual over auto-compact:
- Auto-compact happens at arbitrary points, often mid-task.
- Strategic compacting preserves context through logical phases (compact after
  exploration before execution, after a milestone before the next).

Two signals, each a pure ``(state) -> Optional[message]`` step:
- Context size (primary): the latest assistant `usage` record from the session
  transcript, compared against a window-scaled token threshold
  (COMPACT_CONTEXT_THRESHOLD; default 160k on a 200k window, 250k on 1M),
  re-reminding after every COMPACT_CONTEXT_INTERVAL tokens of growth (default 60k).
  The reminder re-arms when the context falls back below the threshold (e.g. after
  the very /compact this hook recommends), so later growth reminds again.
- Tool-call count (secondary): first at COMPACT_THRESHOLD (default 50), then
  every compact_tool_repeat_interval calls (default 25). Tool count is a weak
  proxy for window pressure — a few large reads can fill the window in very few
  calls, and many tiny calls can cross 50 while the window is barely used.

The numeric defaults above live in the sibling config.json (loaded with stdlib
`json`); each is resolved as env var > config.json > a built-in fallback, so the
hook still works unchanged if config.json is absent or unreadable.

This is a self-contained port of the original Node hook: the `lib/utils` and
`lib/transcript-context` helpers it imported are inlined below so the hook can
live entirely inside the skill directory with no sibling dependencies and no
third-party packages (stdlib only).

Hook contract: always exit 0. Any failure is swallowed and logged to stderr so
a hook error never blocks a tool call.
"""
from __future__ import annotations

import json
import os
import re
import sys
import tempfile
import threading
import time
from collections.abc import Mapping
from pathlib import Path
from typing import NamedTuple

COUNTER_FILE_PREFIX = "claude-tool-count-"
CONTEXT_BUCKET_FILE_PREFIX = "claude-context-bucket-"
STATE_FILE_PREFIXES = (COUNTER_FILE_PREFIX, CONTEXT_BUCKET_FILE_PREFIX)
# Sanity clamp for the per-session counter/bucket files: a corrupted file could
# hold an absurd value, so anything above this is treated as garbage and reset.
MAX_SANE_COUNTER = 1_000_000

# Numeric defaults live in the sibling config.json so they are tunable without
# editing this script. Each lookup is env var > config.json > the built-in
# fallback baked into CONFIG_FALLBACK below (used when the file is absent or
# unreadable). Loading is stdlib `json` only — no third-party deps.
CONFIG_FILE = Path(__file__).resolve().parent.parent / "config.json"
CONFIG_FALLBACK = {
    "compact_threshold": 50,
    "compact_context_threshold_200k": 160_000,
    "compact_context_threshold_1m": 250_000,
    "compact_context_interval": 60_000,
    "compact_state_ttl_days": 14,
    "compact_tool_repeat_interval": 25,
}


def load_config() -> dict:
    """Numeric defaults from the sibling config.json, falling back to the
    hardcoded CONFIG_FALLBACK if the file is missing or unreadable. Per-key
    fallback too, so a partial/garbled config never drops a default. Never
    throws — honours the always-exit-0 hook contract."""
    merged = dict(CONFIG_FALLBACK)
    try:
        data = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return merged
    if isinstance(data, Mapping):
        for key in CONFIG_FALLBACK:
            value = data.get(key)
            if isinstance(value, int) and not isinstance(value, bool):
                merged[key] = value
    return merged


CONFIG = load_config()


# --- inlined lib/utils -------------------------------------------------------

def get_temp_dir() -> str:
    return tempfile.gettempdir()


def write_file(file_path: str, content: str) -> None:
    with open(file_path, "w", encoding="utf-8") as f:
        f.write(content)


def log(msg: str) -> None:
    """Write to stderr (debug log only; does not reach the model)."""
    print(msg, file=sys.stderr)


def output(obj: dict) -> None:
    """Emit one JSON payload to stdout (the model-visible channel)."""
    print(json.dumps(obj))


def read_stdin_json(timeout: float = 1.0) -> dict:
    """Read and parse the hook's stdin JSON payload, with a timeout.

    Returns {} on timeout, empty/interactive stdin, or invalid JSON. The read
    runs on a daemon thread so a stuck pipe can't hang the hook — cross-platform,
    unlike select() on Windows.
    """
    if sys.stdin is None:
        return {}
    try:
        if sys.stdin.isatty():
            return {}
    except Exception:
        pass

    holder: dict[str, str] = {}

    def _read() -> None:
        try:
            holder["raw"] = sys.stdin.read()
        except Exception:
            holder["raw"] = ""

    t = threading.Thread(target=_read, daemon=True)
    t.start()
    t.join(timeout)

    raw = holder.get("raw")
    if not raw:
        return {}
    try:
        parsed = json.loads(raw)
        return parsed if isinstance(parsed, dict) else {}
    except ValueError:
        return {}


def env_int(
    env: Mapping[str, str],
    key: str,
    default: int,
    *,
    min_value: int | None = None,
    max_value: int | None = None,
) -> int:
    """Parse ``env[key]`` as a bounded int.

    Falls back to ``default`` when the value is missing, empty, non-numeric, or
    outside the inclusive [min_value, max_value] range. Centralises the parse/
    validate/default idiom shared by every COMPACT_* environment variable.
    """
    raw = env.get(key)
    if raw in (None, ""):
        return default
    try:
        val = int(raw)
    except ValueError:
        return default
    if min_value is not None and val < min_value:
        return default
    if max_value is not None and val > max_value:
        return default
    return val


# --- inlined lib/transcript-context ------------------------------------------

class ContextUsage(NamedTuple):
    """The latest transcript usage record: context size and the model that produced it."""

    tokens: int
    model: str


def read_tail_lines(file_path: str, max_bytes: int = 256_000) -> list[str]:
    """Return the lines from (at most) the last ``max_bytes`` of ``file_path``.

    The transcript grows for the whole session and only the latest usage record
    is needed, so reading the tail keeps this O(max_bytes) instead of O(file) on
    every tool call. When the byte window starts mid-file, the first (likely
    truncated) line is dropped.
    """
    with open(file_path, "rb") as f:
        f.seek(0, os.SEEK_END)
        size = f.tell()
        start = max(0, size - max_bytes)
        f.seek(start)
        data = f.read()
    lines = data.decode("utf-8", errors="ignore").splitlines()
    if start > 0 and lines:
        del lines[0]
    return lines


def read_latest_context_tokens(transcript_path: str) -> ContextUsage | None:
    """Return the most recent transcript usage record, or None if there isn't one.

    tokens = input + cache_read + cache_creation (the true context size of the
    turn). Only the tail of the (unbounded, ever-growing) transcript is read, and
    it is scanned from the end so the latest assistant turn is found cheaply.
    """
    if not transcript_path:
        return None
    try:
        lines = read_tail_lines(transcript_path)
    except OSError:
        return None

    for line in reversed(lines):
        line = line.strip()
        if not line:
            continue
        try:
            entry = json.loads(line)
        except ValueError:
            continue
        if not isinstance(entry, dict):
            continue
        message = entry.get("message")
        if not isinstance(message, dict):
            message = {}
        usage = message.get("usage")
        if not isinstance(usage, dict):
            usage = entry.get("usage")
        if not isinstance(usage, dict):
            continue
        tokens = (
            int(usage.get("input_tokens") or 0)
            + int(usage.get("cache_read_input_tokens") or 0)
            + int(usage.get("cache_creation_input_tokens") or 0)
        )
        if tokens <= 0:
            continue
        model = message.get("model") or entry.get("model") or ""
        return ContextUsage(tokens=tokens, model=str(model))
    return None


def resolve_context_window_tokens(tokens: int, model: str) -> int:
    """200k by default; 1M when the model carries a `[1m]` marker, or when the
    observed context already exceeds 200k (inferred large window)."""
    if "[1m]" in (model or "").lower():
        return 1_000_000
    if tokens > 200_000:
        return 1_000_000
    return 200_000


def resolve_context_threshold(env: Mapping[str, str], window_tokens: int) -> int:
    """COMPACT_CONTEXT_THRESHOLD if set (0 disables the signal); otherwise the
    window-scaled config.json default (compact_context_threshold_200k on a 200k
    window, compact_context_threshold_1m on a 1M window)."""
    default = (
        CONFIG["compact_context_threshold_1m"]
        if window_tokens >= 1_000_000
        else CONFIG["compact_context_threshold_200k"]
    )
    return env_int(env, "COMPACT_CONTEXT_THRESHOLD", default, min_value=0)


def resolve_context_interval(env: Mapping[str, str]) -> int:
    return env_int(
        env, "COMPACT_CONTEXT_INTERVAL", CONFIG["compact_context_interval"], min_value=1
    )


def compute_context_bucket(tokens: int, threshold: int, interval: int) -> int:
    """Bucket index the context falls into: -1 below threshold, 0 at the
    threshold, then +1 per `interval` tokens of growth."""
    if threshold <= 0 or tokens < threshold:
        return -1
    if interval <= 0:
        return 0
    return (tokens - threshold) // interval


def format_window_label(window_tokens: int) -> str:
    if window_tokens >= 1_000_000:
        m = window_tokens / 1_000_000
        return f"{int(m)}M" if m == int(m) else f"{m:.1f}M"
    return f"{window_tokens // 1000}k"


# --- state helpers -----------------------------------------------------------

def get_counter_retention_days(env: Mapping[str, str]) -> int:
    return env_int(
        env, "COMPACT_STATE_TTL_DAYS", CONFIG["compact_state_ttl_days"], min_value=1
    )


def resolve_tool_threshold(env: Mapping[str, str]) -> int:
    return env_int(
        env, "COMPACT_THRESHOLD", CONFIG["compact_threshold"], min_value=1, max_value=10_000
    )


def cleanup_old_counters(
    temp_dir: str, retention_days: int, current_state_files: list[str]
) -> None:
    """Sweep stale per-session state files from the temp dir.

    Each session writes `claude-tool-count-<id>` (and `claude-context-bucket-<id>`)
    into the OS temp dir; nothing else removes them. This drops files whose mtime
    is older than `retention_days`, preserving the active session's files. Never
    throws — per the always-exit-0 contract any FS failure is logged and skipped.
    """
    try:
        entries = os.listdir(temp_dir)
    except OSError as err:
        log(f"[StrategicCompact] Skipping counter sweep; listdir failed: {err}")
        return

    cutoff = time.time() - retention_days * 24 * 60 * 60
    current = {os.path.basename(p) for p in current_state_files}

    for name in entries:
        if not name.startswith(STATE_FILE_PREFIXES):
            continue
        if name in current:
            continue
        full = os.path.join(temp_dir, name)
        try:
            if not os.path.isfile(full):
                continue
            mtime = os.path.getmtime(full)
        except OSError:
            continue
        # Strict "older than": a file exactly on the cutoff is not older than
        # retention_days, so preserve it. Only mtime < cutoff is deleted.
        if mtime >= cutoff:
            continue
        try:
            os.remove(full)
        except OSError as err:
            log(f"[StrategicCompact] Warning: failed to prune stale counter {full}: {err}")


def increment_tool_call_count(counter_file: str) -> int:
    """Increment and persist the per-session tool-call counter."""
    count = 1
    try:
        with open(counter_file, "r", encoding="utf-8") as f:
            parsed = int(f.read().strip())
        if 0 < parsed <= MAX_SANE_COUNTER:
            count = parsed + 1
    except (OSError, ValueError):
        count = 1
    try:
        write_file(counter_file, str(count))
    except OSError:
        pass
    return count


def read_last_context_bucket(bucket_file: str) -> int:
    """Last context bucket this session fired for (-1 if unfired/unreadable)."""
    try:
        with open(bucket_file, "r", encoding="utf-8") as f:
            parsed = int(f.read().strip())
        return parsed if 0 <= parsed <= MAX_SANE_COUNTER else -1
    except (OSError, ValueError):
        return -1


def maybe_context_suggestion(
    transcript_path: str, bucket_file: str, env: Mapping[str, str]
) -> str | None:
    """Context-size suggestion when the session crosses into a new bucket, else
    None (no transcript, below threshold, disabled, or already fired). Never throws.

    Records the highest bucket fired in ``bucket_file`` and re-arms it (resets to
    -1) when the context falls back below the threshold — e.g. after the very
    /compact this hint recommends — so growth afterwards triggers fresh reminders
    instead of staying silent until the previous peak is exceeded.
    """
    try:
        usage = read_latest_context_tokens(transcript_path)
        if usage is None:
            return None

        window_tokens = resolve_context_window_tokens(usage.tokens, usage.model)
        threshold = resolve_context_threshold(env, window_tokens)
        if threshold <= 0:  # COMPACT_CONTEXT_THRESHOLD=0 disables
            return None

        interval = resolve_context_interval(env)
        bucket = compute_context_bucket(usage.tokens, threshold, interval)
        if bucket < 0:
            # Context shrank back below the threshold (likely a compaction):
            # re-arm so the next climb past the threshold reminds again.
            if read_last_context_bucket(bucket_file) >= 0:
                write_file(bucket_file, "-1")
            return None

        if bucket <= read_last_context_bucket(bucket_file):
            return None

        write_file(bucket_file, str(bucket))

        approx = f"{round(usage.tokens / 1000)}k"
        percent = round((usage.tokens / window_tokens) * 100)
        return (
            f"[StrategicCompact] Context ~{approx} tokens ({percent}% of "
            f"{format_window_label(window_tokens)} window) - consider /compact "
            f"at the next logical boundary"
        )
    except Exception as err:
        log(f"[StrategicCompact] Context signal skipped: {err}")
        return None


def tool_count_message(count: int, threshold: int) -> str | None:
    """Tool-call suggestion: first at ``threshold``, then every
    ``compact_tool_repeat_interval`` calls after (config.json default 25)."""
    if count == threshold:
        return (
            f"[StrategicCompact] {threshold} tool calls reached - "
            "consider /compact if transitioning phases"
        )
    repeat = CONFIG["compact_tool_repeat_interval"]
    if count > threshold and repeat > 0 and (count - threshold) % repeat == 0:
        return (
            f"[StrategicCompact] {count} tool calls - "
            "good checkpoint for /compact if context is stale"
        )
    return None


def main() -> None:
    input_data = read_stdin_json(timeout=1.0)

    raw_session = input_data.get("session_id")
    if not (isinstance(raw_session, str) and raw_session):
        raw_session = os.environ.get("CLAUDE_SESSION_ID") or "default"
    session_id = re.sub(r"[^a-zA-Z0-9_-]", "", raw_session) or "default"

    transcript_path = input_data.get("transcript_path")
    if not isinstance(transcript_path, str):
        transcript_path = ""

    temp_dir = get_temp_dir()
    counter_file = os.path.join(temp_dir, f"{COUNTER_FILE_PREFIX}{session_id}")
    bucket_file = os.path.join(temp_dir, f"{CONTEXT_BUCKET_FILE_PREFIX}{session_id}")

    count = increment_tool_call_count(counter_file)

    # Sweep stale state files once per session (first call) rather than on every
    # tool call: cheap, swallows errors, and preserves the active session's files.
    if count == 1:
        cleanup_old_counters(
            temp_dir, get_counter_retention_days(os.environ), [counter_file, bucket_file]
        )

    # Each signal is (state) -> Optional[str]: the primary reads the real context
    # size from the transcript; the secondary keys off the per-session tool count.
    messages = [
        msg
        for msg in (
            maybe_context_suggestion(transcript_path, bucket_file, os.environ),
            tool_count_message(count, resolve_tool_threshold(os.environ)),
        )
        if msg
    ]

    # Non-blocking PreToolUse stderr (exit 0) is only written to the debug log and
    # does not reach the model. To inject a user-facing suggestion without blocking,
    # emit one JSON payload to stdout with hookSpecificOutput.additionalContext —
    # the documented PreToolUse mechanism. Both signals share the single payload.
    if messages:
        for msg in messages:
            log(msg)
        output(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "additionalContext": "\n".join(messages),
                }
            }
        )

    sys.exit(0)


if __name__ == "__main__":
    try:
        main()
    except Exception as err:  # never let a hook error block a tool call
        print(f"[StrategicCompact] Error: {err}", file=sys.stderr)
        sys.exit(0)
