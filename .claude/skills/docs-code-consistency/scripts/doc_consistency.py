#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Mechanical doc<->code checks for the docs-code-consistency verifier.

This handles only the *deterministic* half of the verdict — the existence and
resolution checks the agent must never eyeball. The judgement half (does the prose
actually describe the behaviour?) stays in SKILL.md. Two subcommands:

  scan   Parse markdown docs (README, docs/, ADRs, CONTEXT.md) and report every
         local reference that does NOT resolve: broken markdown links, dead
         heading anchors, backticked file paths and fenced-command file targets
         that no longer exist on disk. High precision on purpose — under-extract
         rather than emit false positives (see SKILL.md's iron rule).

  probe  Confirm presence/absence of named tokens (symbols, flags, paths) in the
         CODE before the agent flags "documented X no longer exists". Greps the
         repo for each literal token and reports present/absent + a sample hit.
         This is the existence-proof step the iron rule requires.

Usage:
  doc_consistency.py scan [DOC_PATH ...] [--repo ROOT] [--json]
  doc_consistency.py probe TOKEN [TOKEN ...] [--repo ROOT] [--json]
  doc_consistency.py probe --stdin [--repo ROOT] [--json]   # one token per line

Neither subcommand decides the verdict or assigns severity — it reports facts the
agent turns into grounded, located drift items.
"""

import argparse
import json
import re
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Literal, NamedTuple, TypeAlias, cast

CONFIG = Path(__file__).resolve().parent.parent / "config.json"


def _config() -> dict[str, Any]:
    """Per-repo tunables (doc discovery, default severity). Falls back to the
    defaults below if the sibling config.json is missing or unreadable — same
    pattern as thermo-nuclear-code-quality-review's configured_threshold()."""
    try:
        return cast("dict[str, Any]", json.loads(CONFIG.read_text()))
    except (OSError, ValueError):
        return {}


CODE_EXTS = {
    ".py", ".rs", ".ts", ".tsx", ".js", ".jsx", ".go", ".java", ".kt", ".kts",
    ".rb", ".sh", ".bash", ".zsh", ".c", ".h", ".cpp", ".hpp", ".cs", ".php",
    ".swift", ".scala", ".clj", ".ex", ".exs", ".sql", ".md", ".rst", ".txt",
    ".json", ".toml", ".yaml", ".yml", ".ini", ".cfg", ".lock",
}
# Doc discovery defaults — overridable per repo via config.json so the no-arg
# scan still finds docs in projects that don't keep them in docs/ or README*.
_CONFIG = _config()  # read once at import; both defaults below come from it
DOC_GLOBS = tuple(_CONFIG.get("doc_globs") or
                  ("README*.md", "README*.rst", "CONTEXT*.md", "CHANGELOG*.md"))
DOC_DIRS = tuple(_CONFIG.get("doc_dirs") or ("docs",))
RUNNERS = {
    "python", "python3", "py", "bash", "sh", "zsh", "node", "deno", "bun",
    "ruby", "perl", "uv", "uvx", "pytest", "ts-node", "tsx",
}
SKIP_SCHEMES = ("http://", "https://", "mailto:", "ftp://", "tel:", "#")

LINK_RE = re.compile(r"\[(?:[^\]]*)\]\(([^)]+)\)")
BACKTICK_RE = re.compile(r"`([^`\n]+)`")
FENCE_RE = re.compile(r"^\s*```+\s*([\w+-]*)\s*$")
HEADING_RE = re.compile(r"^#{1,6}\s+(.*?)\s*#*\s*$")

# Markdown reference kinds, one per row of the report.
Kind: TypeAlias = Literal["link", "path", "command-target", "read-error"]


@dataclass(frozen=True)
class Ref:
    """One unresolved reference located in a doc — the unit of the scan report."""

    file: str
    line: int
    kind: Kind
    raw: str
    note: str


class Resolution(NamedTuple):
    """Whether a referenced target resolved, plus a note when it did not."""

    resolved: bool
    note: str


def _rel(path: Path, repo: Path) -> Path:
    """Path relative to `repo` when it lives under it, else the path unchanged.
    Lets report locations stay repo-relative without assuming containment."""
    return path.relative_to(repo) if path.is_relative_to(repo) else path


def slugify(heading: str) -> str:
    """GitHub-style anchor slug: lowercase, drop punctuation, spaces -> hyphens."""
    s = heading.strip().lower()
    s = re.sub(r"[^\w\s-]", "", s)
    return re.sub(r"\s+", "-", s)


def looks_like_path(tok: str) -> bool:
    """Conservative: only treat a token as a path worth resolving when it has a
    known code/config extension. Avoids flagging `PASS/FAIL`, `red/green`, etc."""
    tok = tok.strip().strip("'\"")
    if not tok or " " in tok or tok.startswith(SKIP_SCHEMES):
        return False
    suffix = Path(tok.split("#", 1)[0]).suffix.lower()
    return suffix in CODE_EXTS


def discover_docs(targets: list[str], repo: Path) -> list[Path]:
    out: list[Path] = []
    if targets:
        for t in targets:
            p = (repo / t) if not Path(t).is_absolute() else Path(t)
            if p.is_file():
                out.append(p)
            elif p.is_dir():
                out += sorted(q for q in p.rglob("*.md") if q.is_file())
                out += sorted(q for q in p.rglob("*.rst") if q.is_file())
        return sorted(set(out))
    for pat in DOC_GLOBS:
        out += sorted(repo.glob(pat))
    for d in DOC_DIRS:
        dd = repo / d
        if dd.is_dir():
            out += sorted(q for q in dd.rglob("*.md") if q.is_file())
            out += sorted(q for q in dd.rglob("*.rst") if q.is_file())
    return sorted(set(out))


def headings_of(path: Path) -> set[str]:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return set()
    return {slugify(m.group(1)) for line in text.splitlines() if (m := HEADING_RE.match(line))}


def resolve_target(target: str, doc: Path, repo: Path) -> Resolution:
    """Check a file path + optional #anchor and report whether it resolves."""
    path_part, _, anchor = target.partition("#")
    path_part = path_part.strip()
    if not path_part:  # pure in-page anchor
        ok = slugify(anchor) in headings_of(doc) or not anchor
        return Resolution(ok, "" if ok else f"no heading anchor '#{anchor}' in this file")
    candidates = [(doc.parent / path_part), (repo / path_part)]
    hit = next((c for c in candidates if c.exists()), None)
    if hit is None:
        return Resolution(False, "target file does not exist")
    if anchor and hit.suffix.lower() in (".md", ".rst"):
        if slugify(anchor) not in headings_of(hit):
            return Resolution(False, f"file exists but no heading anchor '#{anchor}'")
    return Resolution(True, "")


def scan_doc(doc: Path, repo: Path) -> list[Ref]:
    try:
        lines = doc.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as e:
        return [Ref(str(doc), 0, "read-error", str(e), "could not read file")]
    refs: list[Ref] = []
    in_fence = False
    fence_lang = ""
    rel = _rel(doc, repo)
    for i, line in enumerate(lines, 1):
        if fm := FENCE_RE.match(line):
            in_fence, fence_lang = (False, "") if in_fence else (True, fm.group(1).lower())
            continue
        if in_fence:
            if fence_lang in ("", "bash", "sh", "shell", "console", "zsh", "shellsession"):
                refs += _fenced_paths(line, i, doc, repo, rel)
            continue
        for m in LINK_RE.finditer(line):
            target = m.group(1).split()[0].strip()  # drop optional "title"
            if target.startswith(SKIP_SCHEMES):
                continue
            if not (res := resolve_target(target, doc, repo)).resolved:
                refs.append(Ref(str(rel), i, "link", target, res.note))
        for m in BACKTICK_RE.finditer(line):
            tok = m.group(1).strip()
            if looks_like_path(tok) and not (res := resolve_target(tok, doc, repo)).resolved:
                refs.append(Ref(str(rel), i, "path", tok, res.note))
    return refs


def _fenced_paths(line: str, lineno: int, doc: Path, repo: Path, rel: Path) -> list[Ref]:
    out: list[Ref] = []
    seen: set[str] = set()
    toks = re.split(r"\s+", line.strip())
    for j, tok in enumerate(toks):
        tok = tok.strip().strip("\\").rstrip(";")
        cand = ""
        if tok.startswith("./") or tok.startswith("../"):
            cand = tok
        elif looks_like_path(tok):
            cand = tok
        elif j > 0 and toks[j - 1].rsplit("/", 1)[-1] in RUNNERS and ("/" in tok or looks_like_path(tok)):
            cand = tok
        if not cand or cand in seen:
            continue
        seen.add(cand)
        if not (res := resolve_target(cand, doc, repo)).resolved:
            out.append(Ref(str(rel), lineno, "command-target", cand, res.note))
    return out


def cmd_scan(args: argparse.Namespace) -> int:
    repo = Path(args.repo).resolve()
    docs = discover_docs(args.targets, repo)
    unresolved = [ref for d in docs for ref in scan_doc(d, repo)]
    if args.json:
        print(json.dumps({
            "repo": str(repo),
            "docs_scanned": [str(_rel(d, repo)) for d in docs],
            "unresolved": [asdict(u) for u in unresolved],
        }, indent=2))
        return 0
    print(f"scanned {len(docs)} doc file(s) under {repo}")
    if not docs:
        print("(no markdown/rst docs found — pass explicit paths if they live elsewhere)")
        return 0
    if not unresolved:
        print("no unresolved references found.")
        return 0
    print(f"\n{len(unresolved)} unresolved reference(s) — confirm each is real drift, "
          "not an illustrative/negative example, before recording:\n")
    width = max(len(u.file) for u in unresolved)
    for u in sorted(unresolved, key=lambda u: (u.file, u.line)):
        print(f"{u.file:<{width}}  L{u.line:<4}  [{u.kind}]  {u.raw}  — {u.note}")
    return 0


# --- probe: each token resolves to exactly one of these outcomes -----------------


@dataclass(frozen=True)
class Found:
    token: str
    sample: str  # "path:line" of the first hit


@dataclass(frozen=True)
class Missing:
    token: str


@dataclass(frozen=True)
class Errored:
    token: str
    error: str  # the search itself failed — NOT proof of absence


ProbeResult: TypeAlias = Found | Missing | Errored


def have_rg() -> bool:
    try:
        return subprocess.run(["rg", "--version"], capture_output=True).returncode == 0
    except OSError:
        return False


def grep_token(token: str, repo: Path, use_rg: bool) -> ProbeResult:
    """Literal search for a token in the code.

    rg/grep exit 0 = match, 1 = no match, >=2 = error. An error must NOT be read
    as proof of absence — the iron rule rests on this — so it returns an Errored
    result (distinct from a clean Missing)."""
    argv = (
        ["rg", "--fixed-strings", "--line-number", "--max-count", "1",
         "--no-heading", "--color", "never", token, str(repo)]
        if use_rg else
        ["grep", "-rnF", "--max-count=1",
         "--exclude-dir=.git", "--exclude-dir=node_modules",
         "--exclude-dir=target", "--exclude-dir=.venv", token, str(repo)]
    )
    proc = subprocess.run(argv, capture_output=True, text=True)
    if proc.returncode >= 2:
        return Errored(token, proc.stderr.strip()[:160] or "search error")
    hits = proc.stdout.strip().splitlines()
    if not hits:
        return Missing(token)
    try:
        path, ln, _ = hits[0].split(":", 2)
        return Found(token, f"{_rel(Path(path), repo)}:{ln}")
    except ValueError:
        return Found(token, hits[0][:120])


def probe_json(result: ProbeResult) -> dict[str, Any]:
    match result:
        case Found(token, sample):
            return {"token": token, "present": True, "sample": sample}
        case Missing(token):
            return {"token": token, "present": False, "sample": ""}
        case Errored(token, error):
            return {"token": token, "present": None, "sample": "", "error": error}


def probe_mark_detail(result: ProbeResult) -> tuple[str, str]:
    match result:
        case Found(_, sample):
            return "PRESENT", sample
        case Missing(_):
            return "ABSENT ", ""
        case Errored(_, error):
            return "ERROR  ", error


def cmd_probe(args: argparse.Namespace) -> int:
    repo = Path(args.repo).resolve()
    tokens = list(args.tokens)
    if args.stdin:
        tokens += [ln.strip() for ln in sys.stdin if ln.strip()]
    tokens = list(dict.fromkeys(tokens))  # de-dup, keep order
    if not tokens:
        print("error: no tokens to probe (pass tokens or --stdin)", file=sys.stderr)
        return 1
    use_rg = have_rg()
    results = [grep_token(t, repo, use_rg) for t in tokens]
    if args.json:
        print(json.dumps({"repo": str(repo), "results": [probe_json(r) for r in results]}, indent=2))
        return 0
    width = max(len(r.token) for r in results)
    for r in results:
        mark, detail = probe_mark_detail(r)
        print(f"{mark}  {r.token:<{width}}  {detail}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="Mechanical doc<->code existence checks.")
    sub = ap.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("scan", help="report unresolved references in markdown docs")
    s.add_argument("targets", nargs="*", help="doc files/dirs (default: README, docs/, CONTEXT*)")
    s.add_argument("--repo", default=".", help="repo root (default: cwd)")
    s.add_argument("--json", action="store_true")

    p = sub.add_parser("probe", help="confirm tokens exist (or not) in the code")
    p.add_argument("tokens", nargs="*", help="symbols / flags / paths to confirm")
    p.add_argument("--repo", default=".", help="repo root (default: cwd)")
    p.add_argument("--stdin", action="store_true", help="also read one token per line from stdin")
    p.add_argument("--json", action="store_true")

    args = ap.parse_args()
    match args.cmd:
        case "scan":
            return cmd_scan(args)
        case "probe":
            return cmd_probe(args)
        case _:  # unreachable: subparser is required
            ap.error(f"unknown command: {args.cmd}")


if __name__ == "__main__":
    sys.exit(main())
