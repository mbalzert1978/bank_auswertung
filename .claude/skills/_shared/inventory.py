#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Inventory the skills under a scope path.

Usage: inventory.py <scope-path>

<scope-path> is either a single skill directory (one that contains SKILL.md)
or a directory of skill subdirectories (e.g. .claude/skills/).
A leading ~ is expanded.

Prints a JSON array, one object per skill:
  name         frontmatter `name` (falls back to the directory name)
  description  frontmatter `description`, or null if absent
  arguments    frontmatter `arguments`, or null if the skill declares none
  path         absolute path to the skill directory; symlinks are NOT
               dereferenced, so the path reflects the scope you queried
  has_scripts  whether a scripts/ directory is bundled
  has_assets   whether an assets/ directory is bundled
  has_config   whether a config.json is bundled

Single source of truth. This file lives in skills/_shared/; every skill-meta
skill that needs an inventory step (skill-audit, skill-tune-up, verifier-audit,
build-verifier, propose-skills) reaches it through a relative symlink at its own
scripts/inventory.py. Edit it HERE, not the symlinks. The PEP 723 block above
lets `uv run` provide a 3.10+ interpreter through the symlink (frontmatter
parsing and discovery live in the sibling skill_meta module). Run it instead of
re-deriving skill discovery and bundled-file detection by hand. Skills
referenced only in memory won't appear here — add those separately.
"""

import json
import sys
from pathlib import Path
from typing import Any

from skill_meta import SkillInfo, discover


def as_record(info: SkillInfo) -> dict[str, Any]:
    return {
        "name": info.name,
        "description": info.description,
        "arguments": info.arguments,
        # .absolute(), not .resolve(): keep the queried location rather than
        # dereferencing symlinks.
        "path": str(info.directory.absolute()),
        "has_scripts": info.has_scripts,
        "has_assets": info.has_assets,
        "has_config": info.has_config,
    }


def main() -> int:
    match sys.argv[1:]:
        case [scope_arg]:
            scope = Path(scope_arg).expanduser()
        case _:
            print(__doc__, file=sys.stderr)
            return 2
    if not scope.exists():
        print(f"error: scope path does not exist: {scope}", file=sys.stderr)
        return 1
    print(json.dumps([as_record(s) for s in discover(scope)], indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
