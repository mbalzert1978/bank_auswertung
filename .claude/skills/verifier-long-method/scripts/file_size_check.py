#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Report line counts for the files under review and flag oversized ones.

The mechanical pre-check for verifier-long-method: Long Method is fundamentally a
length smell ("any method longer than ten lines should make you start asking
questions" — refactoring.guru), so run this first and treat OVER/CROSSED rows as
candidates worth opening rather than eyeballing size by hand. This measures whole
files, not individual methods — a much lower threshold than solid-principles-check's
file-size pre-check, tuned so a flag plausibly means "one method is dominating this
file", not "this file has many legitimate responsibilities". Still just a proxy: a
flagged file may turn out to be one cohesive long method, several short ones, or a
long-but-single-purpose sequence — the skill's own judgment call decides which.

Two modes, chosen automatically:
  * git mode (default)  — the files changed on the current branch vs a base ref.
                          Reports each file's OLD and NEW line counts, so a
                          threshold *crossing* (old <= limit < new) is visible,
                          not just absolute size.
  * paths mode          — when every positional arg is an existing file/dir,
                          measures those directly (NEW count only). Matches
                          invoking the review on a specific file or folder.

Usage:
  file_size_check.py [base-ref | path ...] [--threshold N] [--json]

The threshold defaults to `file_size_warn_lines` in the sibling config.json
(or 60 if that is missing/unreadable); --threshold overrides it.

The measurement/classification logic lives in _shared/file_size_check.py —
solid-principles-check's and thermo-nuclear-code-quality-review's own file-size
pre-checks wrap the same module, so the one objective rule has a single source of
truth across all three skills.
"""

import argparse
import json
import sys
from dataclasses import asdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from file_size_check import GitError, changed_via_git, classify, measure_paths
from skill_config import load_config

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"


def main() -> int:
    ap = argparse.ArgumentParser(description="Flag changed files crossing the length threshold.")
    ap.add_argument("targets", nargs="*", help="base ref (git mode) or file/dir paths (paths mode)")
    ap.add_argument("--threshold", type=int, help="override the configured line threshold")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of a table")
    args = ap.parse_args()

    threshold = args.threshold if args.threshold is not None else int(load_config(CONFIG).get("file_size_warn_lines", 60))

    if args.targets and all(Path(t).exists() for t in args.targets):
        measurements = measure_paths(args.targets)
    else:
        match changed_via_git(args.targets[0] if args.targets else None):
            case GitError(message):
                suffix = " (pass file paths to measure them directly)" if message == "not inside a git work tree" else ""
                print(f"error: {message}{suffix}", file=sys.stderr)
                return 1
            case measurements:
                pass

    rows = [classify(m, threshold) for m in measurements]
    if args.json:
        files = [{**asdict(r), "flag": r.flag.value} for r in rows]
        print(json.dumps({"threshold": threshold, "files": files}, indent=2))
        return 0

    if not rows:
        print(f"(no files under review; threshold {threshold} lines)")
        return 0
    width = max(len(r.path) for r in rows)
    print(f"threshold: {threshold} lines\n")
    print(f"{'file':<{width}}  {'old':>6}  {'new':>6}  flag")
    for r in sorted(rows, key=lambda r: r.new or 0, reverse=True):
        flag = r.flag.value
        old = "-" if r.old is None else r.old
        new = "-" if r.new is None else r.new
        print(f"{r.path:<{width}}  {old:>6}  {new:>6}  {flag}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
