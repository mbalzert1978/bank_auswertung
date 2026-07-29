#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Mechanical grounding for architecture-adr-check: list ADRs, list/rank issues.

Does NOT decide relevance or compliance — that's the judgement pass in the
skill. This only extracts what's on disk so the agent isn't hand-transcribing
titles and frontmatter, and ranks issues by a crude keyword overlap with the
changed paths as a starting point, not a filter.

Usage:
  find_relevant_docs.py adrs [--json]
  find_relevant_docs.py issues [--status open] [--match PATH ...] [--json]
"""

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

try:
    sys.stdout.reconfigure(encoding="utf-8")
except (AttributeError, ValueError):
    pass

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from skill_config import load_config

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config.json"

STOPWORDS = {"und", "der", "die", "das", "fuer", "mit", "von", "auf", "ein", "eine", "the", "and", "for", "with"}


def configured_dirs() -> tuple[Path | None, Path | None]:
    cfg = load_config(CONFIG)
    adr_dir = cfg.get("adr_dir")
    issues_dir = cfg.get("issues_dir")
    return (Path(adr_dir) if adr_dir else None, Path(issues_dir) if issues_dir else None)


def tokens(text: str) -> set[str]:
    return {t for t in re.split(r"[^a-zA-Z0-9]+", text.lower()) if len(t) > 2 and t not in STOPWORDS}


@dataclass(frozen=True)
class Adr:
    id: str
    file: str
    title: str


def list_adrs(adr_dir: Path) -> list[Adr]:
    out = []
    for f in sorted(adr_dir.glob("*.md")):
        m = re.match(r"(\d+)-", f.name)
        adr_id = m.group(1) if m else f.stem
        first_line = next((l.strip("# ").strip() for l in f.read_text(encoding="utf-8").splitlines() if l.strip()), f.stem)
        out.append(Adr(adr_id, str(f), first_line))
    return out


@dataclass(frozen=True)
class Issue:
    id: str
    file: str
    title: str
    status: str
    score: int


def parse_frontmatter(text: str) -> dict[str, str]:
    m = re.match(r"^---\n(.*?)\n---", text, flags=re.DOTALL)
    if not m:
        return {}
    fields: dict[str, str] = {}
    for line in m.group(1).splitlines():
        if ":" in line:
            key, _, value = line.partition(":")
            fields[key.strip()] = value.strip().strip('"')
    return fields


def list_issues(issues_dir: Path, status: str | None, match_tokens: set[str]) -> list[Issue]:
    out = []
    for f in sorted(issues_dir.glob("*.md")):
        if f.name == "PROGRESS.md":
            continue
        fm = parse_frontmatter(f.read_text(encoding="utf-8"))
        issue_status = fm.get("status", "unknown")
        if status and issue_status != status:
            continue
        title = fm.get("title", f.stem)
        # Substring overlap, not exact match: German compounds ("BatchTausch")
        # won't split the same way a path segment ("Batch") does.
        title_tokens = tokens(title)
        score = sum(
            1 for t in title_tokens for m in match_tokens if t in m or m in t
        ) if match_tokens else 0
        out.append(Issue(fm.get("id", f.stem), str(f), title, issue_status, score))
    out.sort(key=lambda i: i.score, reverse=True)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)

    adrs_p = sub.add_parser("adrs")
    adrs_p.add_argument("--json", action="store_true")

    issues_p = sub.add_parser("issues")
    issues_p.add_argument("--status")
    issues_p.add_argument("--match", nargs="*", default=[])
    issues_p.add_argument("--json", action="store_true")

    args = ap.parse_args()

    adr_dir, issues_dir = configured_dirs()

    if adr_dir is None or issues_dir is None:
        missing = [n for n, v in (("adr_dir", adr_dir), ("issues_dir", issues_dir)) if v is None]
        print(f"error: {', '.join(missing)} not configured in config.json", file=sys.stderr)
        return 1

    if args.cmd == "adrs":
        rows = list_adrs(adr_dir)
        if args.json:
            print(json.dumps([asdict(r) for r in rows], indent=2))
        else:
            for r in rows:
                print(f"{r.id}  {r.title}  ({r.file})")
        return 0

    match_tokens: set[str] = set()
    for p in args.match:
        match_tokens |= tokens(p)
    rows = list_issues(issues_dir, args.status, match_tokens)
    if args.json:
        print(json.dumps([asdict(r) for r in rows], indent=2))
    else:
        for r in rows:
            print(f"{r.id}  score={r.score}  status={r.status}  {r.title}  ({r.file})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
