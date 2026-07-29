#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""decay_sweep: the Phase-3 date arithmetic for the reflect skill, scripted.

Requires Python 3.10+ (match statements, PEP 604 `X | None` unions). The PEP 723
metadata block above lets `uv run` provision a suitable interpreter regardless of
the system default — invoke via `uv run scripts/decay_sweep.py …` (see SKILL.md).

The model must never eyeball `today - last_triggered`. This script owns that
deterministic half of Phase 3: it scans the experiences dir for `.md` files,
parses each file's frontmatter, computes `days_since` against today, applies the
configured thresholds, and prints a JSON report. SKILL.md *acts* on the report
(moving files, editing the index); the script never touches the filesystem beyond
reading.

Report shape:

  { "today": "YYYY-MM-DD",
    "experiences_dir": "/abs/path",
    "thresholds": { "archive_after_days": 90, "stale_flag_after_days": 60,
                    "archive_min_frequency": 3 },
    "archive":     [ {file, name, days_since, frequency}, ... ],
    "flag_stale":  [ {file, name, days_since, frequency}, ... ],
    "unarchive":   [ {file, name, days_since, frequency}, ... ],
    "skipped":     [ {file, reason}, ... ] }

Decision rules (per non-`archived/` file, skipping `decay_eligible: false`):
  archive    if days_since > archive_after_days AND frequency < archive_min_frequency
  flag_stale if days_since > stale_flag_after_days (and not already archive-bound)
The `archived/` subdir is scanned too, but only to surface recovery candidates as
`unarchive` (entries that are NOT stale — `days_since <= stale_flag_after_days`),
mirroring the Phase-1 un-archive path.

Thresholds and the experiences dir come from the bundled config.json; `--dir`
overrides the dir (it is resolved relative to --memory-dir / cwd when relative).

Usage:
  decay_sweep.py [--memory-dir DIR] [--dir EXPERIENCES_DIR] [--config cfg.json]
                 [--today YYYY-MM-DD] [--pretty]

Exit code is 0 on success, 1 on a usage/config error.
"""

import argparse
import json
import sys
from dataclasses import dataclass
from datetime import date
from pathlib import Path
from typing import Any, Literal, NoReturn, TypeAlias

HERE = Path(__file__).resolve().parent
DEFAULT_CONFIG = HERE.parent / "config.json"

ARCHIVED_DIRNAME = "archived"


def die(msg: str) -> NoReturn:
    print(f"decay-sweep: error: {msg}", file=sys.stderr)
    sys.exit(1)


# --- Domain model: immutable records, illegal states unrepresentable ----------


@dataclass(frozen=True)
class Config:
    experiences_dir: str
    archive_after_days: int
    stale_flag_after_days: int
    archive_min_frequency: int


@dataclass(frozen=True)
class Experience:
    """A parsed experience file: its location plus the frontmatter fields decay
    cares about. `archived` records which side of the archive boundary it sits on,
    so the same record drives both the archive sweep and recovery."""

    file: str  # path relative to the experiences dir
    name: str
    frequency: int
    days_since: int
    decay_eligible: bool
    archived: bool


@dataclass(frozen=True)
class Skipped:
    file: str
    reason: str


# A scanned file is either a usable Experience or a Skipped one (unparseable /
# decay-exempt — those never appear in any action bucket).
Scanned: TypeAlias = Experience | Skipped

Action: TypeAlias = Literal["archive", "flag_stale", "unarchive", "none"]


# --- Config -------------------------------------------------------------------


def load_config(path: Path) -> Config:
    if not path.is_file():
        die(f"config file not found: {path}")
    try:
        if not isinstance(raw := json.loads(path.read_text()), dict):
            die(f"config {path} must be a JSON object")
    except json.JSONDecodeError as e:
        die(f"could not parse config {path}: {e}")

    def as_int(key: str, default: int) -> int:
        if not isinstance(value := raw.get(key, default), int) or isinstance(value, bool):
            die(f"config {key} must be an integer, got {value!r}")
        return value

    return Config(
        experiences_dir=str(raw.get("experiences_dir") or "experiences"),
        archive_after_days=as_int("archive_after_days", 90),
        stale_flag_after_days=as_int("stale_flag_after_days", 60),
        archive_min_frequency=as_int("archive_min_frequency", 3),
    )


# --- Frontmatter parsing ------------------------------------------------------


def parse_frontmatter(text: str) -> dict[str, str] | None:
    """Return the `key: value` map from a leading `---`-fenced YAML block, or None
    when the file has no frontmatter. Only flat scalar lines are read — enough for
    the experience schema, no YAML dependency."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None
    fields: dict[str, str] = {}
    for line in lines[1:]:
        if line.strip() == "---":
            return fields
        key, sep, value = line.partition(":")
        if sep:
            fields[key.strip()] = value.strip()
    return None  # no closing fence


def parse_bool(value: str) -> bool:
    return value.strip().lower() in ("true", "yes", "1")


def parse_date(value: str) -> date | None:
    try:
        return date.fromisoformat(value.strip())
    except ValueError:
        return None


def scan_file(path: Path, root: Path, today: date, *, archived: bool) -> Scanned:
    """Parse one experience file into an Experience, or a Skipped explaining why
    (unreadable, no frontmatter, bad/missing `last_triggered`, or decay-exempt)."""
    rel = str(path.relative_to(root))
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        return Skipped(rel, f"could not read file: {e}")
    fm = parse_frontmatter(text)
    if fm is None:
        return Skipped(rel, "no frontmatter block")
    if not parse_bool(fm.get("decay_eligible", "true")):
        return Skipped(rel, "decay_eligible: false")
    if (last := parse_date(fm.get("last_triggered", ""))) is None:
        return Skipped(rel, "missing or invalid last_triggered")
    try:
        frequency = int(fm.get("frequency", "0"))
    except ValueError:
        return Skipped(rel, "invalid frequency")
    return Experience(
        file=rel,
        name=fm.get("name", path.stem),
        frequency=frequency,
        days_since=(today - last).days,
        decay_eligible=True,
        archived=archived,
    )


def scan_dir(root: Path, today: date) -> list[Scanned]:
    """Scan the live experiences dir and its `archived/` subdir. Live files feed
    the archive/stale buckets; archived files feed recovery (`unarchive`)."""
    out: list[Scanned] = []
    for md in sorted(p for p in root.glob("*.md") if p.is_file()):
        out.append(scan_file(md, root, today, archived=False))
    archived_dir = root / ARCHIVED_DIRNAME
    if archived_dir.is_dir():
        for md in sorted(p for p in archived_dir.glob("*.md") if p.is_file()):
            out.append(scan_file(md, root, today, archived=True))
    return out


# --- Decision -----------------------------------------------------------------


def classify(exp: Experience, cfg: Config) -> Action:
    """Map one experience to its bucket. Archived files can only be recovered;
    live files archive, then flag-stale, else stay put."""
    match exp.archived:
        case True:
            return "unarchive" if exp.days_since <= cfg.stale_flag_after_days else "none"
        case False:
            if exp.days_since > cfg.archive_after_days and exp.frequency < cfg.archive_min_frequency:
                return "archive"
            if exp.days_since > cfg.stale_flag_after_days:
                return "flag_stale"
            return "none"


def entry(exp: Experience) -> dict[str, Any]:
    return {
        "file": exp.file,
        "name": exp.name,
        "days_since": exp.days_since,
        "frequency": exp.frequency,
    }


def build_report(scanned: list[Scanned], cfg: Config, today: date, root: Path) -> dict[str, Any]:
    buckets: dict[Action, list[dict[str, Any]]] = {"archive": [], "flag_stale": [], "unarchive": []}
    skipped: list[dict[str, str]] = []
    for item in scanned:
        match item:
            case Skipped(file, reason):
                skipped.append({"file": file, "reason": reason})
            case Experience() as exp:
                if (action := classify(exp, cfg)) != "none":
                    buckets[action].append(entry(exp))
    return {
        "today": today.isoformat(),
        "experiences_dir": str(root),
        "thresholds": {
            "archive_after_days": cfg.archive_after_days,
            "stale_flag_after_days": cfg.stale_flag_after_days,
            "archive_min_frequency": cfg.archive_min_frequency,
        },
        "archive": buckets["archive"],
        "flag_stale": buckets["flag_stale"],
        "unarchive": buckets["unarchive"],
        "skipped": skipped,
    }


# --- Resolution & entry point -------------------------------------------------


def resolve_dir(cfg: Config, memory_dir: str, override: str | None) -> Path:
    """Resolve the experiences dir: --dir wins, else config's experiences_dir,
    each taken relative to --memory-dir (default cwd) when not absolute."""
    base = Path(memory_dir).expanduser()
    target = Path(override or cfg.experiences_dir).expanduser()
    root = (target if target.is_absolute() else base / target).resolve()
    if not root.is_dir():
        die(f"experiences dir does not exist: {root}")
    return root


def parse_today(value: str | None) -> date:
    if value is None:
        return date.today()
    if (parsed := parse_date(value)) is None:
        die(f"--today must be YYYY-MM-DD, got {value!r}")
    return parsed


def main() -> int:
    ap = argparse.ArgumentParser(prog="decay_sweep.py", description=__doc__.splitlines()[0])
    ap.add_argument("--memory-dir", default=".", help="memory dir relative paths resolve against (default: cwd)")
    ap.add_argument("--dir", default=None, help="experiences dir (default: config experiences_dir)")
    ap.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="config.json (default: bundled next to the skill)")
    ap.add_argument("--today", default=None, help="override today's date as YYYY-MM-DD (default: system date)")
    ap.add_argument("--pretty", action="store_true", help="indent the JSON report")
    args = ap.parse_args()

    cfg = load_config(args.config)
    today = parse_today(args.today)
    root = resolve_dir(cfg, args.memory_dir, args.dir)
    report = build_report(scan_dir(root, today), cfg, today, root)
    print(json.dumps(report, indent=2 if args.pretty else None))
    return 0


if __name__ == "__main__":
    sys.exit(main())
