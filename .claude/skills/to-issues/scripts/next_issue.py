#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Assign the next issue ID, filename, and status for one to-issues slice.

Deterministic companion to /to-issues step 6: ensures docs/issues/PROGRESS.md
exists (bootstrapping it from assets/progress-template.md on first run), scans
the configured issue dir for the highest existing 4-digit ID and increments
it, slugifies --title into the matching filename, and derives status from the
*actual* frontmatter status of each --blocked-by issue -- "open" unless no
blockers were given, or some given blocker isn't already "closed". A blocker
from an earlier run that has since been closed therefore does not force
"blocked".

Run once per slice, in dependency order, so each slice's blockers are already
on disk (with a real status) by the time this runs for it.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from datetime import date
from pathlib import Path
from typing import NoReturn

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"
PROGRESS_TEMPLATE = HERE.parent / "assets" / "progress-template.md"

FRONTMATTER = re.compile(r"\A---\n(.*?)\n---", re.DOTALL)
FIELD = re.compile(r"^\s*(?P<key>[A-Za-z_][\w-]*)\s*:\s*(?P<value>.*)$", re.MULTILINE)
LEADING_ID = re.compile(r"^(\d{4})-")


def die(msg: str) -> NoReturn:
    print(f"next-issue: error: {msg}", file=sys.stderr)
    sys.exit(1)


# --- Config -------------------------------------------------------------------


@dataclass(frozen=True)
class Config:
    issue_dir: str
    progress_file: str


def load_config(path: Path) -> Config:
    if not path.is_file():
        die(f"config file not found: {path}")
    try:
        if not isinstance(raw := json.loads(path.read_text()), dict):
            die(f"config {path} must be a JSON object")
    except json.JSONDecodeError as e:
        die(f"could not parse config {path}: {e}")
    return Config(
        issue_dir=str(raw.get("issue_dir") or "docs/issues"),
        progress_file=str(raw.get("progress_file") or "PROGRESS.md"),
    )


# --- Parsing (pure) -----------------------------------------------------------


def parse_frontmatter(text: str) -> dict[str, str]:
    """Parse the 'key: value' block between the leading '---' fence and the next."""
    if not (block := FRONTMATTER.match(text.replace("\r\n", "\n"))):
        return {}
    return {
        m["key"].lower(): m["value"].strip().strip("\"'")
        for m in FIELD.finditer(block.group(1))
    }


def issue_files(issue_dir: Path, progress_file: str) -> list[Path]:
    if not issue_dir.is_dir():
        return []
    return [p for p in sorted(issue_dir.glob("*.md")) if p.name != progress_file]


def ensure_progress_file(issue_dir: Path, progress_file: str) -> bool:
    """Bootstrap PROGRESS.md from the template if missing. Returns True if created."""
    path = issue_dir / progress_file
    if path.exists():
        return False
    if not PROGRESS_TEMPLATE.is_file():
        die(f"progress template not found: {PROGRESS_TEMPLATE}")
    issue_dir.mkdir(parents=True, exist_ok=True)
    template = PROGRESS_TEMPLATE.read_text(encoding="utf-8")
    path.write_text(template.replace("{{DATE}}", date.today().isoformat()), encoding="utf-8")
    return True


def slugify(title: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")
    if not slug:
        die(f"title '{title}' has no usable characters for a filename slug")
    return slug


def issue_id(path: Path, fields: dict[str, str]) -> str:
    """Frontmatter id, falling back to the filename's leading 4-digit number."""
    if fields.get("id"):
        return fields["id"]
    return m.group(1) if (m := LEADING_ID.match(path.name)) else ""


# --- Domain logic ---------------------------------------------------------------


def next_id(files: list[Path]) -> str:
    ids = [int(m.group(1)) for f in files if (m := LEADING_ID.match(f.name))]
    return f"{(max(ids) + 1) if ids else 1:04d}"


def blocker_status(files: list[Path], blocker_id: str) -> str:
    for f in files:
        fields = parse_frontmatter(f.read_text(encoding="utf-8-sig"))
        if issue_id(f, fields) == blocker_id:
            return fields.get("status") or ""
    die(f"no issue found for blocker ID '{blocker_id}' -- publish blockers before dependents")


def resolve_status(files: list[Path], blocked_by: list[str]) -> str:
    if not blocked_by:
        return "open"
    statuses = [blocker_status(files, b) for b in blocked_by]
    return "open" if all(s == "closed" for s in statuses) else "blocked"


# --- Main -----------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--title", required=True, help="the slice title, used for the filename slug")
    ap.add_argument(
        "--blocked-by", nargs="*", default=[], metavar="ID",
        help="IDs of issues this slice depends on (already published, e.g. earlier in this run)",
    )
    args = ap.parse_args()

    cfg = load_config(CONFIG)
    issue_dir = Path(cfg.issue_dir)
    progress_bootstrapped = ensure_progress_file(issue_dir, cfg.progress_file)
    files = issue_files(issue_dir, cfg.progress_file)

    result = {
        "id": (assigned_id := next_id(files)),
        "filename": f"{assigned_id}-{slugify(args.title)}.md",
        "status": resolve_status(files, args.blocked_by),
        "progress_bootstrapped": progress_bootstrapped,
    }
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
