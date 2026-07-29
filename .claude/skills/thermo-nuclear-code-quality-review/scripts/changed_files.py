#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Report line counts for the files under review and flag oversized ones.

This is the deterministic file-size check for thermo-nuclear-code-quality-review.
The skill's one fully objective rule — "don't let a file cross the line threshold"
— shouldn't depend on the agent eyeballing it. Run this first, then review.

Two modes, chosen automatically:
  * git mode (default)  — the files changed on the current branch vs a base ref.
                          Reports each file's OLD and NEW line counts, so a
                          threshold *crossing* (old <= limit < new) is visible,
                          not just absolute size.
  * paths mode          — when every positional arg is an existing file/dir,
                          measures those directly (NEW count only). Matches
                          invoking the review on a specific file or folder.

Usage:
  changed_files.py [base-ref | path ...] [--threshold N] [--json]

The threshold defaults to `file_size_warn_lines` in the sibling config.json
(or 1000 if that is missing/unreadable); --threshold overrides it.

The measurement/classification logic lives in _shared/file_size_check.py —
solid-principles-check's file_size_check.py wraps the same module, so the one
objective rule has a single source of truth across both skills.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from file_size_check import run_cli

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"


def main() -> int:
    return run_cli(CONFIG, 1000, "Flag changed files crossing the size threshold.")


if __name__ == "__main__":
    sys.exit(main())
