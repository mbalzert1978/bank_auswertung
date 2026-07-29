#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Enumerate candidate test-facade files for the Fowler Test-API-shape check.

This script does NOT judge the shape (fluent arrange, hidden internals, correct
dependency direction) -- that is a semantic property, not a syntactic one, and
regex-matching a specific language's syntax (an earlier version of this script
did that for C#) is both language-locked and fragile (word-boundary bugs on
compound identifiers, false positives on legitimate naming variation). The
judgment is instead made by whichever agent runs the qa-check skill, against
the checklist in its SKILL.md -- the same split architecture-adr-check already
uses (script lists candidates mechanically, agent judges compliance).

This script only answers the mechanical, language-agnostic question: for each
in-scope feature project, does at least one file exist matching the configured
`test_facade_glob`?

Which projects are in scope is configured, not assumed -- a repo may mark its
features by a shared prefix, by a suffix, or by both:

  projects_root             directory the projects live in       (default "src")
  feature_project_prefix    required name prefix, "" for none    (default "")
  feature_project_suffixes  required name suffixes, [] for any   (default [])
  specs_suffix              spec projects, always out of scope   (default ".Specs")
  test_facade_glob          glob, relative to a project dir

Usage:
  test_api_shape.py [--json]

Exit code is 0 on success, 1 on a config error.
"""

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import NoReturn

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from skill_config import load_config

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"


def die(msg: str) -> NoReturn:
    print(f"test-api-shape: error: {msg}", file=sys.stderr)
    sys.exit(1)


@dataclass(frozen=True)
class Scope:
    """Which project directories carry a test facade, and where it lives inside
    them. Constructed only by `load_scope`, so `root` is known to exist."""

    root: Path
    prefix: str
    suffixes: tuple[str, ...]
    specs_suffix: str
    facade_glob: str

    def includes(self, name: str) -> bool:
        if name.endswith(self.specs_suffix) or not name.startswith(self.prefix):
            return False
        return not self.suffixes or any(name.endswith(s) for s in self.suffixes)

    def feature_dirs(self) -> tuple[Path, ...]:
        return tuple(sorted(p for p in self.root.iterdir() if p.is_dir() and self.includes(p.name)))

    def describe(self) -> str:
        prefix = f"prefix {self.prefix!r}" if self.prefix else "any prefix"
        suffixes = f"suffixes {list(self.suffixes)}" if self.suffixes else "any suffix"
        return f"{prefix}, {suffixes}"


@dataclass(frozen=True)
class Candidate:
    feature: str
    files: tuple[str, ...]


def load_scope(path: Path) -> Scope:
    cfg = load_config(path)

    if not (root_raw := str(cfg.get("projects_root") or "src").strip()):
        die("config projects_root must be a non-empty string")
    if not (root := Path(root_raw)).is_dir():
        die(f"configured projects_root does not exist: {root} (run from the repo root)")
    if not isinstance(suffixes := cfg.get("feature_project_suffixes", []), (list, tuple)):
        die(f"config feature_project_suffixes must be a list of strings, got {suffixes!r}")
    if not (facade_glob := str(cfg.get("test_facade_glob") or "").strip()):
        die("config test_facade_glob must be a non-empty glob, relative to a project directory")

    return Scope(
        root=root,
        prefix=str(cfg.get("feature_project_prefix") or ""),
        suffixes=tuple(str(s) for s in suffixes if str(s)),
        specs_suffix=str(cfg.get("specs_suffix") or ".Specs"),
        facade_glob=facade_glob,
    )


def main() -> int:
    ap = argparse.ArgumentParser(description="List test-facade candidates per feature project.")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of a table")
    args = ap.parse_args()

    scope = load_scope(CONFIG)
    if not (feature_dirs := scope.feature_dirs()):
        die(f"no feature projects found under {scope.root} matching {scope.describe()}")

    rows = tuple(
        Candidate(d.name, tuple(sorted(str(p) for p in d.glob(scope.facade_glob))))
        for d in feature_dirs
    )

    if args.json:
        payload = {"glob": scope.facade_glob, "features": [asdict(r) for r in rows]}
        print(json.dumps(payload, indent=2))
        return 0

    for r in rows:
        status = ", ".join(r.files) if r.files else "MISSING (no file matches the configured glob)"
        print(f"{r.feature:<55} {status}")
    print(f"\nscope: {scope.root} ({scope.describe()})")
    print(f"glob: {scope.facade_glob}")
    print(f"missing: {sum(not r.files for r in rows)}")
    print("(features with a match still need agent judgment against the Fowler checklist in SKILL.md)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
