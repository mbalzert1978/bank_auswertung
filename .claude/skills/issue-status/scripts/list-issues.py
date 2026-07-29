#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Canonical issue scanner - DMSSyncv2 /issue-status.

Scans the configured issue dir for *.md (except the progress file), extracts
id/title/status from each file's YAML frontmatter, and emits a JSON array on
stdout. Files without a parseable frontmatter block surface as "(malformed)"
rather than being dropped; their id then falls back to the filename.

No arguments. Working directory is the repo root (harness guarantee); the issue
dir and progress filename come from the sibling config.json.
"""
from __future__ import annotations

import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import NoReturn

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"

FRONTMATTER = re.compile(r"\A---\n(.*?)\n---", re.DOTALL)
FIELD = re.compile(r"^\s*(?P<key>[A-Za-z_][\w-]*)\s*:\s*(?P<value>.*)$", re.MULTILINE)
LEADING_NUMBER = re.compile(r"^(\d+)")

MALFORMED = "(malformed)"


def die(msg: str) -> NoReturn:
    print(f"issue-status: error: {msg}", file=sys.stderr)
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


def id_from_name(stem: str) -> str:
    return m.group(1) if (m := LEADING_NUMBER.match(stem)) else stem


# --- Domain model -------------------------------------------------------------


@dataclass(frozen=True)
class Issue:
    """One issue row. `title`/`status` carry "(malformed)" when no frontmatter
    block parsed, so the report still lists the file instead of dropping it."""

    file: str
    id: str
    title: str
    status: str


def scan(path: Path) -> Issue:
    fields = parse_frontmatter(path.read_text(encoding="utf-8-sig"))
    return Issue(
        file=path.name,
        id=fields.get("id") or id_from_name(path.stem),
        title=fields.get("title") or MALFORMED,
        status=fields.get("status") or MALFORMED,
    )


# --- Main ---------------------------------------------------------------------


def main() -> int:
    cfg = load_config(CONFIG)
    issue_dir = Path(cfg.issue_dir)
    if not issue_dir.is_dir():
        print("[]")
        return 0
    issues = [
        scan(path)
        for path in sorted(issue_dir.glob("*.md"))
        if path.name != cfg.progress_file
    ]
    print(json.dumps([asdict(issue) for issue in issues], indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
