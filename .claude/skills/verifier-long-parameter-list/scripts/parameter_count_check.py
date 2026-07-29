#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Report the size of every parenthesized, comma-separated group in the files
under review, and flag the oversized ones.

The mechanical pre-check for verifier-long-parameter-list: "more than three or
four parameters" (refactoring.guru) is directly countable *without knowing any
language's declaration syntax* — a parenthesized group with commas is common to
every bracket-using language, whether it turns out to be a method signature, a
constructor, a call site, or a tuple. This script deliberately does not try to
recognise "is this a declaration" (that needs per-language keywords, which is
exactly the kind of language lock-in the verifier itself is not supposed to
have) — it just measures group size and lets the skill decide, from the
located line, whether the group is a genuine Long Parameter List candidate or
an unrelated comma-heavy expression (a call with several arguments, a tuple
literal). Nested groups are measured independently, so `Foo(Bar(a, b, c), d)`
reports both the 2-item outer group and the 3-item inner one.

Two modes, chosen automatically:
  * git mode (default)  — the files changed on the current branch vs a base
                          ref, scanned as they stand at HEAD.
  * paths mode          — when every positional arg is an existing file/dir,
                          scans the files under those paths directly.

Usage:
  parameter_count_check.py [base-ref | path ...] [--threshold N] [--json]

The threshold defaults to `max_group_items` in the sibling config.json (or 4
if that is missing/unreadable — refactoring.guru's own "three or four"); a
group with strictly more items than the threshold is flagged.
"""

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from skill_config import load_config
from skill_git import GitError, changed_paths, resolve_merge_base

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"

OPEN_TO_CLOSE = {"(": ")", "[": "]", "{": "}"}
CLOSERS = frozenset(OPEN_TO_CLOSE.values())


@dataclass(frozen=True)
class ParenGroup:
    """One parenthesized group with its item count already computed —
    downstream code only ever sees a complete, already-counted group."""

    file: str
    line: int
    item_count: int
    preview: str


def _skip_string_or_char(text: str, i: int) -> int:
    """If text[i] opens a string/char literal, return the index just past its
    close; otherwise return i unchanged — keeps a comma or bracket quoted
    inside a literal from disturbing the depth count."""
    quote = text[i]
    j = i + 1
    while j < len(text) and text[j] != quote:
        j += 2 if text[j] == "\\" else 1
    return j + 1 if j < len(text) else j


def _matching_close(text: str, open_index: int) -> int:
    """Index just past the bracket that closes text[open_index], tracking
    nested (), [], {} so a comma inside a nested literal isn't mistaken for a
    top-level separator. Returns len(text) if it never closes."""
    opener = text[open_index]
    stack = [OPEN_TO_CLOSE[opener]]
    i = open_index + 1
    while i < len(text) and stack:
        ch = text[i]
        if ch in "\"'":
            i = _skip_string_or_char(text, i)
            continue
        if ch in OPEN_TO_CLOSE:
            stack.append(OPEN_TO_CLOSE[ch])
        elif ch in CLOSERS:
            if ch == stack[-1]:
                stack.pop()
            i += 1
            if not stack:
                return i
            continue
        i += 1
    return len(text)


def _count_top_level_commas(span: str) -> int:
    """Commas at bracket-depth 0 within a group's inner span (nested (), [],
    {} don't count). An empty/whitespace-only span has zero items."""
    if not span.strip():
        return 0
    depth = 0
    commas = 0
    i = 0
    while i < len(span):
        ch = span[i]
        if ch in "\"'":
            i = _skip_string_or_char(span, i)
            continue
        if ch in OPEN_TO_CLOSE:
            depth += 1
        elif ch in CLOSERS:
            depth = max(0, depth - 1)
        elif ch == "," and depth == 0:
            commas += 1
        i += 1
    return commas + 1


def find_paren_groups(text: str) -> list[ParenGroup]:
    """Pure: every `(...)` group in a source string, each with its own item
    count. `file` is filled in by the caller. Groups are found independently
    of nesting, so a long inner call inside a short outer one is still seen."""
    out: list[ParenGroup] = []
    i = 0
    while i < len(text):
        ch = text[i]
        if ch in "\"'":
            i = _skip_string_or_char(text, i)
            continue
        if ch == "(":
            close = _matching_close(text, i)
            span = text[i + 1 : close - 1]
            count = _count_top_level_commas(span)
            if count > 0:
                line = text.count("\n", 0, i) + 1
                preview = " ".join(span.split())
                preview = preview[:57] + "..." if len(preview) > 60 else preview
                out.append(ParenGroup("", line, count, preview))
            i += 1
        else:
            i += 1
    return out


def exceeds(group: ParenGroup, threshold: int) -> bool:
    return group.item_count > threshold


# --- file discovery ---------------------------------------------------------

# Build/VCS output, not source in any language — skipped for speed and to
# avoid scanning generated noise, not because of anything language-specific.
SKIP_DIR_NAMES = frozenset({".git", "bin", "obj", "node_modules"})


def _is_binary(path: Path) -> bool:
    """A NUL byte in the first 8 KiB is the standard cheap binary sniff test —
    no text source file legitimately contains one."""
    try:
        return b"\0" in path.open("rb").read(8192)
    except OSError:
        return True


def _files_under(paths: list[str]) -> list[str]:
    out: list[str] = []
    for t in paths:
        p = Path(t)
        if p.is_file():
            if not _is_binary(p):
                out.append(str(p))
        elif p.is_dir():
            for q in sorted(p.rglob("*")):
                if q.is_file() and SKIP_DIR_NAMES.isdisjoint(q.parts) and not _is_binary(q):
                    out.append(str(q))
    return out


def _files_changed(base: str | None) -> list[str] | GitError:
    match resolve_merge_base(base):
        case GitError() as err:
            return err
        case merge_base:
            return list(changed_paths(merge_base))


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    ap = argparse.ArgumentParser(description="Flag oversized parenthesized groups.")
    ap.add_argument("targets", nargs="*", help="base ref (git mode) or file/dir paths (paths mode)")
    ap.add_argument("--threshold", type=int, help="override the configured max item count")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of a table")
    args = ap.parse_args()

    threshold = args.threshold if args.threshold is not None else int(load_config(CONFIG).get("max_group_items", 4))

    if args.targets and all(Path(t).exists() for t in args.targets):
        files = _files_under(args.targets)
    else:
        match _files_changed(args.targets[0] if args.targets else None):
            case GitError(message):
                suffix = " (pass file paths to scan them directly)" if message == "not inside a git work tree" else ""
                print(f"error: {message}{suffix}", file=sys.stderr)
                return 1
            case files:
                pass

    groups: list[ParenGroup] = []
    for f in files:
        try:
            text = Path(f).read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        groups += (ParenGroup(f, g.line, g.item_count, g.preview) for g in find_paren_groups(text))

    flagged = [g for g in groups if exceeds(g, threshold)]

    if args.json:
        print(json.dumps({"threshold": threshold, "groups": [vars(g) for g in flagged]}, indent=2))
        return 0

    if not flagged:
        print(f"(no groups over {threshold} items; {len(groups)} non-empty group(s) scanned)")
        return 0
    width = max(len(f"{g.file}:{g.line}") for g in flagged)
    print(f"threshold: {threshold} items ({len(groups)} non-empty group(s) scanned)\n")
    print(f"{'location':<{width}}  items  preview")
    for g in sorted(flagged, key=lambda g: g.item_count, reverse=True):
        loc = f"{g.file}:{g.line}"
        print(f"{loc:<{width}}  {g.item_count:>5}  ({g.preview})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
