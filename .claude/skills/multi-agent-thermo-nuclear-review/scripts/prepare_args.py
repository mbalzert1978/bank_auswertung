#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Build the `args` payload for the multi-agent-thermo-nuclear-review Workflow script.

The Workflow orchestration script (`multi_agent_review.js`) runs in a sandbox with no
filesystem access, so everything it needs must arrive via the Workflow tool's `args`. This is
the deterministic prep step: read the skill's `config.json` + `assets/`, parse the three
prompt-template blocks, and emit the finished `args` JSON to stdout. The agent captures that
stdout and passes it straight to `Workflow({ ..., args })`.

The config is parsed once into an immutable model (frozen dataclasses); a shape mismatch is a
hard error (SystemExit), not a silent default — this skill ships its own config, so the only
way it is wrong is a genuine edit mistake worth surfacing.

Usage:
  prepare_args.py --scope "<file/dir path | PR number | base-ref/diff range>"
"""

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
SKILL_DIR = HERE.parent
CONFIG = SKILL_DIR / "config.json"
ASSETS = SKILL_DIR / "assets"

# Delegation target — the rubric the finder/verifier agents read for their review standard.
# Repo-relative so the sub-agents (cwd = repo root) can open it directly.
THERMO_STANDARDS_PATH = ".claude/skills/thermo-nuclear-code-quality-review/SKILL.md"

# One named template block per heading: `## NAME` then a fenced block whose body is the template.
_BLOCK = re.compile(r"^##\s+(HEADER|FINDER|VERIFIER)\s*\n+```[^\n]*\n(.*?)\n```", re.MULTILINE | re.DOTALL)


# --- config: immutable, parsed once from config.json -----------------------------


@dataclass(frozen=True)
class Lens:
    name: str
    angles: tuple[str, ...]


@dataclass(frozen=True)
class ReviewConfig:
    finder_count: int
    lenses: tuple[Lens, ...]
    guardrails: tuple[str, ...]


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as e:
        raise SystemExit(f"prepare_args: cannot read {path}: {e}")


def load_json(path: Path) -> object:
    try:
        return json.loads(read_text(path))
    except json.JSONDecodeError as e:
        raise SystemExit(f"prepare_args: invalid JSON in {path}: {e}")


def parse_lens(index: int, obj: object) -> Lens:
    match obj:
        case {"name": str(name), "angles": [str(), *_] as angles}:
            return Lens(name, tuple(angles))
        case _:
            raise SystemExit(f"prepare_args: config.lenses[{index}] needs a string 'name' and a non-empty 'angles' array of strings")


def load_config(path: Path) -> ReviewConfig:
    match load_json(path):
        case {
            "finder_count": int(finder_count),
            "lenses": [dict(), *_] as lenses,
            "guardrails": [*guardrails],
        } if finder_count > 0:
            return ReviewConfig(
                finder_count=finder_count,
                lenses=tuple(parse_lens(i, lens) for i, lens in enumerate(lenses)),
                guardrails=tuple(guardrails),
            )
        case _:
            raise SystemExit("prepare_args: config.json must have finder_count (int > 0), a non-empty 'lenses' array, and a 'guardrails' array")


# --- assets: prompt templates ----------------------------------------------------


def load_templates(path: Path) -> dict[str, str]:
    blocks = {name: body.strip() for name, body in _BLOCK.findall(read_text(path))}
    missing = [n for n in ("HEADER", "FINDER", "VERIFIER") if not blocks.get(n)]
    if missing:
        raise SystemExit(f"prepare_args: {path.name} is missing template block(s): {', '.join(missing)}")
    return {"header": blocks["HEADER"], "finder": blocks["FINDER"], "verifier": blocks["VERIFIER"]}


# --- main ------------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description="Emit the args JSON for the multi-agent review workflow.")
    ap.add_argument("--scope", required=True, help="what to review: a path, a PR number, or a base-ref/diff range")
    args = ap.parse_args()

    config = load_config(CONFIG)
    payload = {
        "scope": args.scope,
        "lenses": [{"name": lens.name, "angles": list(lens.angles)} for lens in config.lenses],
        "finder_count": config.finder_count,
        "guardrails": "\n".join(f"- {g}" for g in config.guardrails),
        "thermoStandardsPath": THERMO_STANDARDS_PATH,
        "templates": load_templates(ASSETS / "prompt-templates.md"),
        "finderSchema": load_json(ASSETS / "finder-schema.json"),
        "verifierSchema": load_json(ASSETS / "verifier-schema.json"),
    }
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
