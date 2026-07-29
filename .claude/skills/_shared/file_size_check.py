"""Shared file-size measurement/classification logic.

Used by thermo-nuclear-code-quality-review's changed_files.py and
solid-principles-check's file_size_check.py — both are thin argparse CLI
wrappers around this module, so the one deterministic rule ("don't let a
file cross the line threshold") has one source of truth.

Two modes, chosen automatically by the caller:
  * git mode (default)  — the files changed on the current branch vs a base ref.
                          Reports each file's OLD and NEW line counts, so a
                          threshold *crossing* (old <= limit < new) is visible,
                          not just absolute size.
  * paths mode          — when every positional arg is an existing file/dir,
                          measures those directly (NEW count only).
"""

from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import NamedTuple

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
