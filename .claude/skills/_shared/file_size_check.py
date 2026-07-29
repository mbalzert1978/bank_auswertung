"""Shared file-size measurement/classification logic.

Used by verifier-large-class's, verifier-long-method's,
solid-principles-check's and thermo-nuclear-code-quality-review's own
file-size pre-checks — all four are thin argparse CLI wrappers around this
module (via `run_cli`), so the one deterministic rule ("don't let a file
cross the line threshold") has one source of truth.

Two modes, chosen automatically by the caller:
  * git mode (default)  — the files changed on the current branch vs a base ref.
                          Reports each file's OLD and NEW line counts, so a
                          threshold *crossing* (old <= limit < new) is visible,
                          not just absolute size.
  * paths mode          — when every positional arg is an existing file/dir,
                          measures those directly (NEW count only).
"""

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from enum import Enum
from pathlib import Path
from typing import NamedTuple

from skill_config import load_config
from skill_git import GitError, changed_paths, git, resolve_merge_base


class Measurement(NamedTuple):
    """A file under review with its line counts; `old` is None in paths mode."""

    path: str
    old: int | None
    new: int | None


class SizeFlag(Enum):
    """The exclusive, closed set of threshold verdicts a file can carry."""

    NONE = ""
    OVER = "OVER"
    CROSSED = "CROSSED"


@dataclass(frozen=True)
class FileRow:
    """A measurement plus its threshold verdict — the unit of the report."""

    path: str
    old: int | None
    new: int | None
    flag: SizeFlag


def count_lines(path: Path) -> int | None:
    try:
        with path.open("rb") as f:
            return sum(1 for _ in f)
    except OSError:
        return None


def changed_via_git(base: str | None) -> list[Measurement] | GitError:
    match resolve_merge_base(base):
        case GitError() as err:
            return err
        case merge_base:
            pass
    root = Path(git("rev-parse", "--show-toplevel").stdout.strip())
    out: list[Measurement] = []
    for f in changed_paths(merge_base):
        old = git("show", f"{merge_base}:{f}")
        out.append(Measurement(
            f,
            len(old.stdout.splitlines()) if old.returncode == 0 else 0,
            count_lines(root / f),
        ))
    return out


def measure_paths(targets: list[str]) -> list[Measurement]:
    out: list[Measurement] = []
    for t in targets:
        p = Path(t)
        files = [p] if p.is_file() else sorted(
            q for q in p.rglob("*") if q.is_file() and ".git/" not in f"{q}/"
        )
        out += (Measurement(str(q), None, count_lines(q)) for q in files)
    return out


def classify(m: Measurement, threshold: int) -> FileRow:
    exceeds = m.new is not None and m.new > threshold
    crossed = m.old is not None and m.old <= threshold < (m.new or 0)
    flag = SizeFlag.CROSSED if crossed else SizeFlag.OVER if exceeds else SizeFlag.NONE
    return FileRow(m.path, m.old, m.new, flag)


def run_cli(config_path: Path, default_threshold: int, description: str) -> int:
    """Shared CLI entry point for every file-size-check wrapper.

    `config_path` is the caller's own sibling `config.json` (resolved by the
    caller, relative to *its* `__file__` — this module has no opinion on
    where a wrapper lives) and `default_threshold` is the fallback used when
    that config is missing/unreadable or lacks `file_size_warn_lines`.
    `description` is the wrapper's own argparse description string.
    """
    ap = argparse.ArgumentParser(description=description)
    ap.add_argument("targets", nargs="*", help="base ref (git mode) or file/dir paths (paths mode)")
    ap.add_argument("--threshold", type=int, help="override the configured line threshold")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of a table")
    args = ap.parse_args()

    threshold = args.threshold if args.threshold is not None else int(
        load_config(config_path).get("file_size_warn_lines", default_threshold)
    )

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
