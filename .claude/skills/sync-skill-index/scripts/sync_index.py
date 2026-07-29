#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Regenerate the skill-inventory region of CLAUDE.md from the filesystem.

Usage:
  sync_index.py                 # use bundled config.json, write CLAUDE.md
  sync_index.py /path/cfg.json  # use a specific config
  sync_index.py --check         # don't write; exit 1 if CLAUDE.md is stale

What it does (deterministic, same way every time):
  1. Scans the configured skills_dir for */SKILL.md (via the shared skill_meta).
  2. Builds the directory-tree block (names + which of scripts/ assets/
     config.json each skill bundles) — fully derived from disk.
  3. Groups skills into buckets using the name->bucket map in config.json,
     rendering one summary line per skill (override from config, else the
     first sentence of the SKILL.md description).
  4. Fills assets/inventory.template.md and splices it into CLAUDE.md between
     the begin/end markers, leaving the rest of the file untouched.

Classification is the one thing the filesystem can't tell us, so it lives in
config.json. A skill on disk but absent from the map is an ERROR (classify it
first); a map entry no longer on disk is a WARNING (and is skipped).
"""

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

from skill_meta import SkillInfo, discover

HERE = Path(__file__).resolve().parent
SKILL_DIR = HERE.parent
DEFAULT_CONFIG = SKILL_DIR / "config.json"
TEMPLATE = SKILL_DIR / "assets" / "inventory.template.md"


# --- config: immutable, parsed once from the JSON --------------------------------


@dataclass(frozen=True)
class Bucket:
    name: str
    tagline: str


@dataclass(frozen=True)
class Classification:
    bucket: str
    summary: str | None  # explicit override; falls back to the description


@dataclass(frozen=True)
class Config:
    skills_dir: Path
    claude_md: Path
    tree_root_label: str
    begin_marker: str
    end_marker: str
    buckets: tuple[Bucket, ...]
    skills: dict[str, Classification]  # skill name -> its classification


def load_config(path: Path) -> Config:
    raw = json.loads(path.expanduser().read_text(encoding="utf-8"))
    return Config(
        skills_dir=Path(raw["skills_dir"]).expanduser(),
        claude_md=Path(raw["claude_md"]).expanduser(),
        tree_root_label=raw["tree_root_label"],
        begin_marker=raw["begin_marker"],
        end_marker=raw["end_marker"],
        buckets=tuple(Bucket(b["name"], b["tagline"]) for b in raw["buckets"]),
        skills={
            name: Classification(meta["bucket"], meta.get("summary"))
            for name, meta in raw["skills"].items()
        },
    )


# --- rendering: pure functions over the discovered skills -------------------------


def extras_of(info: SkillInfo) -> list[str]:
    """Bundled sub-paths, in the fixed display order scripts/ assets/ config.json."""
    flags = ((info.has_scripts, "scripts/"), (info.has_assets, "assets/"), (info.has_config, "config.json"))
    return [label for present, label in flags if present]


def first_sentence(description: str | None) -> str:
    """Condense a description into a summary when config gives no override.

    Drops a leading wrapping quote, then takes everything before the `Use when`
    trigger clause or the first sentence break — whichever comes first.
    """
    text = (description or "").strip().strip('"').strip()
    for cut in (". Use when", " Use when"):
        if (idx := text.find(cut)) != -1:
            text = text[:idx]
            break
    if (period := text.find(". ")) != -1:
        text = text[:period]
    return text.rstrip(". ").strip()


def render_tree(root_label: str, skills: dict[str, SkillInfo]) -> str:
    names = sorted(skills)
    width = max((len(n) + 1 for n in names), default=0)  # name + trailing "/"
    lines = [root_label]
    for name in names:
        entry = (name + "/").ljust(width)
        extras = "".join(f"  {x}" for x in extras_of(skills[name]))
        lines.append(f"  {entry} SKILL.md{extras}")
    lines.append("CLAUDE.md")
    return "\n".join(lines)


def render_buckets(cfg: Config, skills: dict[str, SkillInfo]) -> tuple[str, list[str]]:
    if unclassified := sorted(set(skills) - set(cfg.skills)):
        bucket_names = ", ".join(b.name for b in cfg.buckets)
        raise SystemExit(
            "error: these skills are on disk but unclassified in config.json:\n"
            + "".join(f"  - {n}\n" for n in unclassified)
            + f'add each under "skills" with a bucket ({bucket_names}) and rerun.'
        )

    warnings = [
        f"stale config entry (no SKILL.md on disk): {name} — skipped"
        for name in sorted(set(cfg.skills) - set(skills))
    ]

    sections = []
    for bucket in cfg.buckets:
        members = sorted(
            name
            for name, meta in cfg.skills.items()
            if meta.bucket == bucket.name and name in skills
        )
        if not members:
            continue
        lines = [f"**{bucket.name}** — {bucket.tagline}:", ""]
        for name in members:
            summary = cfg.skills[name].summary or first_sentence(skills[name].description)
            lines.append(f"- `{name}` — {summary}")
        sections.append("\n".join(lines))
    return "\n\n".join(sections), warnings


def splice(claude_md: Path, begin: str, end: str, region: str) -> tuple[str, str]:
    original = claude_md.read_text(encoding="utf-8")
    if begin not in original or end not in original:
        raise SystemExit(
            f"error: markers not found in {claude_md}.\n"
            f"add these two lines around the generated inventory region:\n"
            f"  {begin}\n  ...\n  {end}"
        )
    b = original.index(begin) + len(begin)
    e = original.index(end)
    updated = original[:b] + "\n" + region + "\n" + original[e:]
    return original, updated


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("config", nargs="?", default=str(DEFAULT_CONFIG))
    ap.add_argument("--check", action="store_true",
                    help="don't write; exit 1 if CLAUDE.md is out of date")
    args = ap.parse_args()

    cfg = load_config(Path(args.config))
    if not cfg.skills_dir.is_dir():
        raise SystemExit(f"error: skills_dir does not exist: {cfg.skills_dir}")

    skills = {info.name: info for info in discover(cfg.skills_dir)}
    tree = render_tree(cfg.tree_root_label, skills)
    buckets, warnings = render_buckets(cfg, skills)
    region = (TEMPLATE.read_text(encoding="utf-8")
              .replace("{{TREE}}", tree)
              .replace("{{BUCKETS}}", buckets)
              .strip("\n"))

    original, updated = splice(cfg.claude_md, cfg.begin_marker, cfg.end_marker, region)

    for w in warnings:
        print(f"warning: {w}", file=sys.stderr)

    counts = {b.name: 0 for b in cfg.buckets}
    for name in skills:
        counts[cfg.skills[name].bucket] += 1
    summary = ", ".join(f"{n}: {c}" for n, c in counts.items())
    tally = f"{len(skills)} skills; {summary}"

    match args.check, original == updated:
        case True, True:
            print(f"OK — CLAUDE.md inventory is current ({tally})")
            return 0
        case True, False:
            print(f"STALE — CLAUDE.md inventory is out of date ({tally})")
            return 1
        case False, True:
            print(f"unchanged — CLAUDE.md already current ({tally})")
            return 0
        case False, False:
            cfg.claude_md.write_text(updated, encoding="utf-8")
            print(f"updated {cfg.claude_md} ({tally})")
            return 0
    raise AssertionError("unreachable")  # the 2x2 match above is exhaustive


if __name__ == "__main__":
    sys.exit(main())
