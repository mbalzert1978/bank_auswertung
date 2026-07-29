#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Flag changed production projects with no accompanying change in their
co-located specs project.

This is a heuristic, not a coverage percentage: it only proves "something in
the sibling specs project changed too", not that the right behavior is tested.
Judge the actual test content yourself; this script just finds candidates.

The project layout is configured, not assumed, so the same script serves a
`src/Foo` + `src/Foo.Specs` repo and a `crates/foo_adapter` + `crates/foo_specs`
one:

  projects_root        directory the projects live in           (default "src")
  specs_suffix         what marks a project as the spec sibling (default ".Specs")
  production_suffixes  suffixes marking a project as production (default [""])

`production_suffixes` also carries the sibling mapping: the matched suffix is
stripped to get the stem, and the sibling is stem + specs_suffix. With the
default [""] nothing is stripped, so `Foo` -> `Foo.Specs`; with
["_domain", "_adapter"] both `foo_domain` and `foo_adapter` -> `foo_specs`.

Usage:
  coverage_gap.py [base-ref] [--json]

Prints one row per changed production project: covered (its specs sibling also
changed), gap (production changed, the sibling exists on disk but did not
change), or no-specs-sibling (no such project exists — not a gap, there is
nothing to require).

Exit code is 0 on success, 1 on a config or git error.
"""

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from enum import Enum
from pathlib import Path
from typing import NoReturn

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from skill_config import load_config
from skill_git import GitError, changed_paths, resolve_merge_base

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"


def die(msg: str) -> NoReturn:
    print(f"coverage-gap: error: {msg}", file=sys.stderr)
    sys.exit(1)


class CoverageStatus(Enum):
    """The exclusive, closed set of coverage verdicts a changed project can carry."""

    COVERED = "covered"
    GAP = "gap"
    NO_SPECS_SIBLING = "no-specs-sibling"


@dataclass(frozen=True)
class Layout:
    """How this repo names production projects and their spec siblings.
    Constructed only by `load_layout`, so the suffixes are already validated and
    ordered longest-first — a catch-all "" can never shadow a real suffix."""

    root: Path
    specs_suffix: str
    production_suffixes: tuple[str, ...]

    def stem_of(self, project: str) -> str | None:
        """The stem a specs sibling is derived from, or None if `project` is not
        a production project under this layout."""
        if project.endswith(self.specs_suffix):
            return None  # this IS a specs project, not something requiring one
        for suffix in self.production_suffixes:
            if project.endswith(suffix):
                return project[: len(project) - len(suffix)] if suffix else project
        return None

    def specs_of(self, project: str) -> str | None:
        match self.stem_of(project):
            case None:
                return None
            case stem:
                return stem + self.specs_suffix

    def project_of(self, path: str) -> str | None:
        """The project a changed path belongs to, or None if it lies outside the
        configured root."""
        parts = Path(path).parts
        depth = len(self.root.parts)
        if len(parts) <= depth or parts[:depth] != self.root.parts:
            return None
        return parts[depth]


@dataclass(frozen=True)
class ProjectRow:
    project: str
    specs_dir: str
    status: CoverageStatus


def load_layout(path: Path) -> Layout:
    cfg = load_config(path)

    if not (root_raw := str(cfg.get("projects_root") or "src").strip()):
        die("config projects_root must be a non-empty string")
    if not (root := Path(root_raw)).is_dir():
        die(f"configured projects_root does not exist: {root} (run from the repo root)")
    if not (specs_suffix := str(cfg.get("specs_suffix") or ".Specs")):
        die("config specs_suffix must be a non-empty string")
    if not isinstance(suffixes := cfg.get("production_suffixes", [""]), (list, tuple)):
        die(f"config production_suffixes must be a list of strings, got {suffixes!r}")

    # Longest first so an explicit suffix always wins over a catch-all "".
    ordered = tuple(sorted((str(s) for s in suffixes), key=len, reverse=True))
    return Layout(root, specs_suffix, ordered or ("",))


def classify(project: str, specs_dir: str, layout: Layout, touched: set[str]) -> CoverageStatus:
    match ((layout.root / specs_dir).is_dir(), specs_dir in touched):
        case (False, _):
            return CoverageStatus.NO_SPECS_SIBLING
        case (True, True):
            return CoverageStatus.COVERED
        case _:
            return CoverageStatus.GAP


def rows_for(touched: set[str], layout: Layout) -> tuple[ProjectRow, ...]:
    """Pure: one row per changed production project, specs projects skipped."""
    return tuple(
        ProjectRow(project, specs_dir, classify(project, specs_dir, layout, touched))
        for project in sorted(touched)
        if (specs_dir := layout.specs_of(project)) is not None
    )


def changed_projects(base: str | None, layout: Layout) -> set[str] | GitError:
    match resolve_merge_base(base):
        case GitError() as err:
            return err
        case merge_base:
            paths = changed_paths(merge_base)
    return {p for f in paths if (p := layout.project_of(f.strip()))}


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Flag changed production projects with no accompanying specs change."
    )
    ap.add_argument("base", nargs="?", help="base ref to diff against (default: auto-resolved)")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of a table")
    args = ap.parse_args()

    layout = load_layout(CONFIG)
    match changed_projects(args.base, layout):
        case GitError() as err:
            die(err.message)
        case touched:
            rows = rows_for(touched, layout)

    if args.json:
        print(json.dumps([{**asdict(r), "status": r.status.value} for r in rows], indent=2))
        return 0

    if not rows:
        print(f"(no {layout.root}/ production projects changed)")
    for r in rows:
        print(f"{r.project:<55} {r.status.value}")
    print(f"\ngaps: {sum(r.status is CoverageStatus.GAP for r in rows)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
