"""Shared skill-metadata primitives: read SKILL.md frontmatter and discover skills.

Single source of truth for the three things every skill-meta script does the same
way: parse a SKILL.md's frontmatter, find the skills under a scope, and report
which of scripts/ assets/ config.json each one bundles. It lives in
skills/_shared/ alongside inventory.py.

This is an importable *library*, never run directly, so it carries no PEP 723 block
or shebang. Consumers import it as a sibling module:

  * a script symlinked from _shared/ (e.g. inventory.py) imports it for free —
    Python resolves the symlinked __main__ to its real dir, putting _shared/ on
    sys.path, so `from skill_meta import …` just works through the symlink.
  * a real script in its own skill (e.g. sync_index.py) reaches it through a
    relative symlink at scripts/skill_meta.py -> ../../_shared/skill_meta.py.

Edit it HERE, not the symlinks.
"""

from __future__ import annotations  # keep this importable under any 3.x interpreter

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Frontmatter:
    """The scalar SKILL.md frontmatter fields the skill-meta tools read.

    Immutable and self-documenting: only the three keys any consumer needs are
    modelled (other scalar keys in the block are parsed but not retained — nothing
    reads them). Each field is None when its key is absent, kept distinct from
    present-but-empty (""), which SkillInfo.name relies on for its fallback.
    """

    name: str | None = None
    description: str | None = None
    arguments: str | None = None


def parse_frontmatter(skill_md: Path) -> Frontmatter:
    """Pull the scalar name/description/arguments fields from the leading
    --- frontmatter block.

    Deliberately minimal (stdlib-only, no PyYAML): top-level scalar keys only.
    Nested mappings (`metadata:`) and YAML list items (`allowed-tools:` entries)
    are not parsed. Returns an empty Frontmatter when no block is present.
    """
    lines = skill_md.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0].strip() != "---":
        return Frontmatter()
    fields: dict[str, str] = {}
    for line in lines[1:]:
        if line.strip() == "---":
            break
        key, sep, value = line.partition(":")
        if sep:
            fields[key.strip()] = value.strip()
    return Frontmatter(
        name=fields.get("name"),
        description=fields.get("description"),
        arguments=fields.get("arguments"),
    )


@dataclass(frozen=True)
class SkillInfo:
    """One discovered skill: its directory and parsed frontmatter.

    `directory` is kept exactly as discovered (not resolved), so callers decide
    whether to dereference symlinks — inventory keeps the queried location on
    purpose. The accessors below are the only fields any consumer needs.
    """

    directory: Path
    frontmatter: Frontmatter

    @property
    def name(self) -> str:
        # Fall back to the directory name only when `name` is absent (None), not
        # when it is present-but-empty ("") — matches the legacy behaviour.
        name = self.frontmatter.name
        return self.directory.name if name is None else name

    @property
    def description(self) -> str | None:
        return self.frontmatter.description

    @property
    def arguments(self) -> str | None:
        return self.frontmatter.arguments

    @property
    def has_scripts(self) -> bool:
        return (self.directory / "scripts").is_dir()

    @property
    def has_assets(self) -> bool:
        return (self.directory / "assets").is_dir()

    @property
    def has_config(self) -> bool:
        return (self.directory / "config.json").exists()


def discover(scope: Path) -> tuple[SkillInfo, ...]:
    """Find every skill under `scope`, sorted by directory.

    `scope` is either a single skill directory (it contains SKILL.md) or a
    directory of skill subdirectories (e.g. .claude/skills/).
    """
    if (scope / "SKILL.md").exists():
        directories = [scope]
    else:
        directories = sorted({md.parent for md in scope.glob("*/SKILL.md")})
    return tuple(SkillInfo(d, parse_frontmatter(d / "SKILL.md")) for d in directories)
