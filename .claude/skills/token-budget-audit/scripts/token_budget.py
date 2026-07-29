#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""token_budget: mechanical size/coverage pass for the token-budget-audit skill.

This handles only the *deterministic* half of the audit — file discovery, size
estimation, peer-outlier detection, and ignore-file coverage. The judgement half
(is the same instruction block duplicated across files? is verbose prose actually
redundant?) stays in SKILL.md, same split as docs-code-consistency's scan/probe.

Walks SCOPE for every CLAUDE.md and SKILL.md, estimates each file's token count
via the shared `token_estimate.estimate_tokens` (see skills/_shared/token_estimate.py
— also used by compress-prompt), using chars_per_token from the sibling config.json,
and flags:
  - a CLAUDE.md over claude_md_token_guideline
  - a SKILL.md more than skill_size_outlier_multiplier times its siblings'
    average size (siblings = other SKILL.md files under the same parent dir)
  - a directory named in config.json's large_dir_names that exists under SCOPE
    but isn't covered by any .gitignore/.claudeignore between SCOPE and the
    repo root

Usage:
  token_budget.py SCOPE

Prints a JSON object with keys: files, claude_md_flags, skill_size_outliers,
uncovered_large_dirs. Exit 0 on success; 2 on a usage error (bad path).
"""

import json
import sys
from pathlib import Path
from typing import Any, cast

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from token_estimate import estimate_tokens  # noqa: E402

CONFIG = Path(__file__).resolve().parent.parent / "config.json"
DEFAULTS = {
    "chars_per_token": 4,
    "claude_md_token_guideline": 1500,
    "skill_size_outlier_multiplier": 4,
    "large_dir_names": ["node_modules", "dist", "build", "bin", "obj", ".venv", "target"],
}


def _config() -> dict[str, Any]:
    try:
        loaded = cast("dict[str, Any]", json.loads(CONFIG.read_text(encoding="utf-8")))
    except (OSError, ValueError):
        loaded = {}
    return {**DEFAULTS, **loaded}


def find_files(scope: Path, name: str) -> list[Path]:
    return sorted(scope.rglob(name))


def find_repo_root(scope: Path) -> Path:
    """Nearest ancestor of `scope` containing `.git`, or `scope` itself if none is found."""
    current = scope.resolve()
    while True:
        if (current / ".git").exists():
            return current
        if current.parent == current:
            return scope.resolve()
        current = current.parent


def ignore_files_above(path: Path, stop_at: Path) -> list[Path]:
    """.gitignore/.claudeignore files from `path`'s directory up to `stop_at` (inclusive)."""
    found: list[Path] = []
    current = (path if path.is_dir() else path.parent).resolve()
    stop_at = stop_at.resolve()
    while True:
        for name in (".claudeignore", ".gitignore"):
            candidate = current / name
            if candidate.is_file():
                found.append(candidate)
        if current == stop_at or current.parent == current:
            break
        current = current.parent
    return found


def is_covered(dir_name: str, ignore_files: list[Path]) -> bool:
    for f in ignore_files:
        try:
            lines = f.read_text(encoding="utf-8", errors="ignore").splitlines()
        except OSError:
            continue
        for line in lines:
            stripped = line.strip().strip("/")
            if stripped == dir_name:
                return True
    return False


def audit(scope: Path) -> dict[str, Any]:
    cfg = _config()
    chars_per_token = cfg["chars_per_token"]
    repo_root = find_repo_root(scope)

    files: list[dict[str, Any]] = []
    claude_md_flags: list[dict[str, Any]] = []
    skill_md_by_parent: dict[Path, list[dict[str, Any]]] = {}

    for path in find_files(scope, "CLAUDE.md"):
        text = path.read_text(encoding="utf-8", errors="ignore")
        tokens = estimate_tokens(text, chars_per_token)
        entry = {"path": str(path), "kind": "CLAUDE.md", "chars": len(text), "estimated_tokens": tokens}
        files.append(entry)
        if tokens > cfg["claude_md_token_guideline"]:
            claude_md_flags.append({**entry, "guideline": cfg["claude_md_token_guideline"]})

    for path in find_files(scope, "SKILL.md"):
        text = path.read_text(encoding="utf-8", errors="ignore")
        tokens = estimate_tokens(text, chars_per_token)
        entry = {"path": str(path), "kind": "SKILL.md", "chars": len(text), "estimated_tokens": tokens}
        files.append(entry)
        skill_md_by_parent.setdefault(path.parent.parent, []).append(entry)

    skill_size_outliers: list[dict[str, Any]] = []
    for siblings in skill_md_by_parent.values():
        if len(siblings) < 2:
            continue
        avg = sum(s["estimated_tokens"] for s in siblings) / len(siblings)
        threshold = avg * cfg["skill_size_outlier_multiplier"]
        for s in siblings:
            if s["estimated_tokens"] > threshold and s["estimated_tokens"] > 0:
                skill_size_outliers.append({**s, "peer_average_tokens": round(avg), "threshold": round(threshold)})

    uncovered_large_dirs: list[str] = []
    for dir_name in cfg["large_dir_names"]:
        for hit in scope.rglob(dir_name):
            if not hit.is_dir():
                continue
            ignore_files = ignore_files_above(hit, repo_root)
            if not is_covered(dir_name, ignore_files):
                uncovered_large_dirs.append(str(hit))

    return {
        "files": files,
        "claude_md_flags": claude_md_flags,
        "skill_size_outliers": skill_size_outliers,
        "uncovered_large_dirs": sorted(set(uncovered_large_dirs)),
    }


def main() -> int:
    match sys.argv[1:]:
        case [scope_arg]:
            scope = Path(scope_arg).expanduser()
        case _:
            print("token_budget: error: usage: token_budget.py SCOPE", file=sys.stderr)
            return 2
    if not scope.is_dir():
        print(f"token_budget: error: not a directory: {scope}", file=sys.stderr)
        return 2
    print(json.dumps(audit(scope), indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
