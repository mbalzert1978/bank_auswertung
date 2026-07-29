#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""auto-commit: collect a repo's changes, then commit grouped changes.

Requires Python 3.10+ (match statements, PEP 604 `X | None` unions). The PEP 723
metadata block above lets `uv run` provision a suitable interpreter regardless of
the system default — invoke via `uv run scripts/auto_commit.py …` (see SKILL.md).

Two subcommands split the work between the model (judgement) and this script
(deterministic git plumbing + message rendering from a template):

  collect   Print every changed path with a status label and its diff vs HEAD,
            so the model can read what changed and group cohesive files.

  commit    Given a plan.json that groups files into commits, render a
            Conventional-Commits message for each group from the template, stage
            exactly that group's files (including untracked & deletions) and
            commit it. Groups are committed in order; files not in any group are
            left uncommitted and reported.

Plan format (see SKILL.md):

  { "commits": [
      { "type": "feat", "scope": "repository",
        "subject": "add A and its repository",
        "body": ["introduce class A", "add ARepository for persistence of A"],
        "files": ["src/A.kt", "src/ARepository.kt"] },
      ...
  ] }

`scope` and `body` are optional. `body` may be a string or a list of lines.
Configured trailers (e.g. Co-Authored-By) are appended to every message.

Usage:
  auto_commit.py collect [--config cfg.json] [--max-diff-lines N] [--no-untracked]
  auto_commit.py commit  --plan plan.json [--config cfg.json] [--dry-run]
                         (use --plan - to read the plan from stdin)

Exit code is 0 on success, 1 on a usage/validation/git error.
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, NamedTuple, NoReturn, TypeAlias

HERE = Path(__file__).resolve().parent
DEFAULT_CONFIG = HERE.parent / "config.json"

DEFAULT_TYPES = (
    "feat",
    "fix",
    "refactor",
    "docs",
    "test",
    "chore",
    "style",
    "perf",
    "build",
    "ci",
)

# git porcelain status codes -> human labels (untracked/rename/copy are modelled
# as their own Change variants below, so they are not in this map).
STATUS_LABELS = {
    "A": "added",
    "M": "modified",
    "D": "deleted",
    "T": "typechange",
    "U": "unmerged",
    "R": "renamed",
    "C": "copied",
}


def die(msg: str) -> NoReturn:
    print(f"auto-commit: error: {msg}", file=sys.stderr)
    sys.exit(1)


# --- Domain model: immutable records, illegal states unrepresentable ----------


@dataclass(frozen=True)
class Git:
    """The git CLI bound to one repo root; every call runs `git -C root …`, so
    the command behaves the same no matter what directory it is invoked from."""

    root: str

    def run(self, *args: str, check: bool = True) -> str:
        res = subprocess.run(
            ["git", "-C", self.root, *args],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        if check and res.returncode != 0:
            die(f"`git {' '.join(args)}` failed:\n{res.stderr.strip()}")
        return res.stdout


@dataclass(frozen=True)
class Untracked:
    path: str


@dataclass(frozen=True)
class Tracked:
    """A path tracked by git: added / modified / deleted / typechange / unmerged."""

    status: str
    path: str


@dataclass(frozen=True)
class Moved:
    """A rename or copy. Always carries its `origin`, so the origin can never
    dangle as an unstaged deletion — and an origin can't attach to anything else."""

    status: str  # "renamed" | "copied"
    path: str
    origin: str


# A changed path is exactly one of these three shapes.
Change: TypeAlias = Untracked | Tracked | Moved


@dataclass(frozen=True)
class Config:
    repo: str
    include_untracked: bool
    max_diff_lines: int
    allowed_types: tuple[str, ...]
    trailers: tuple[str, ...]


@dataclass(frozen=True)
class CommitSpec:
    """A validated commit group from the plan: type and subject are non-empty,
    files is non-empty, body lines are pre-rendered. Constructed only by
    parse_spec, so downstream code never sees an invalid spec."""

    type: str
    subject: str
    files: tuple[str, ...]
    scope: str | None = None
    body: tuple[str, ...] = ()


@dataclass(frozen=True)
class PlannedCommit:
    files: tuple[str, ...]  # primary paths, shown to the user
    stage: tuple[str, ...]  # pathspecs to stage: files + rename/copy origins
    message: str


@dataclass(frozen=True)
class Plan:
    commits: tuple[PlannedCommit, ...]
    uncovered: tuple[str, ...]  # changed paths not assigned to any commit


class MadeCommit(NamedTuple):
    """One commit apply_plan created: its short SHA and rendered subject line."""

    sha: str
    subject: str


# --- Change behaviour: pure functions matching on the union -------------------


def label_of(change: Change) -> str:
    match change:
        case Untracked():
            return "untracked"
        case Tracked(status) | Moved(status):
            return status


def origin_note(change: Change) -> str:
    match change:
        case Moved(origin=origin):
            return f" (from {origin})"
        case _:
            return ""


def stage_paths(change: Change) -> tuple[str, ...]:
    """Pathspecs to stage so the change commits atomically."""
    match change:
        case Moved(path=path, origin=origin):
            return (path, origin)
        case _:
            return (change.path,)


# --- Config & repo resolution -------------------------------------------------


def load_config(path: Path) -> Config:
    if not path.is_file():
        die(f"config file not found: {path}")
    try:
        if not isinstance(raw := json.loads(path.read_text()), dict):
            die(f"config {path} must be a JSON object")
    except json.JSONDecodeError as e:
        die(f"could not parse config {path}: {e}")

    if not isinstance(max_diff_lines := raw.get("max_diff_lines", 400), int):
        die(f"config max_diff_lines must be an integer, got {max_diff_lines!r}")
    if not isinstance(allowed := raw.get("allowed_types") or DEFAULT_TYPES, (list, tuple)):
        die(f"config allowed_types must be a list of strings, got {allowed!r}")
    if not isinstance(trailers_raw := raw.get("trailers") or [], list):
        die(f"config trailers must be a list of strings, got {trailers_raw!r}")

    return Config(
        repo=str(raw.get("repo") or "").strip(),
        include_untracked=bool(raw.get("include_untracked", True)),
        max_diff_lines=max_diff_lines,
        allowed_types=tuple(str(t) for t in allowed),
        trailers=tuple(t for t in trailers_raw if str(t).strip()),
    )


def resolve_repo(cfg: Config) -> Git:
    """Pick the repo to operate on and verify it's a work tree. Uses the absolute
    `repo` from config if set; otherwise auto-detects the work-tree root of the
    current directory. Dies if the result isn't a git work tree."""
    match cfg.repo:
        case "":
            res = subprocess.run(
                ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
            )
            if res.returncode != 0:
                die("not inside a git work tree (and no `repo` set in config)")
            root = res.stdout.strip()
        case configured:
            root = os.path.abspath(os.path.expanduser(configured))
            if not os.path.isdir(root):
                die(f"configured repo path does not exist: {root}")
    git = Git(root)
    if git.run("rev-parse", "--is-inside-work-tree", check=False).strip() != "true":
        die(f"not a git work tree: {root}")
    return git


# --- collect ------------------------------------------------------------------


def parse_changes(raw: str, include_untracked: bool) -> tuple[Change, ...]:
    """Parse `git status --porcelain=v1 -z` into Change records. Pure."""
    tokens = iter(t for t in raw.split("\0") if t)
    changes: list[Change] = []
    for rec in tokens:
        xy, path = rec[:2], rec[3:]
        match xy[0]:
            case "?":
                if include_untracked:
                    changes.append(Untracked(path))
            case "R" | "C" as move_code:
                # The next token is the rename/copy origin.
                changes.append(Moved(STATUS_LABELS[move_code], path, next(tokens, "")))
            case _:
                code = xy[1] if xy[0] == " " else xy[0]
                changes.append(Tracked(STATUS_LABELS.get(code, code), path))
    return tuple(changes)


def collect_changes(git: Git, include_untracked: bool) -> tuple[Change, ...]:
    return parse_changes(git.run("status", "--porcelain=v1", "-z"), include_untracked)


def _truncate(text: str, max_lines: int) -> str:
    lines = text.splitlines()
    if max_lines and len(lines) > max_lines:
        hidden = len(lines) - max_lines
        lines = lines[:max_lines] + [f"... (truncated, {hidden} more lines)"]
    return "\n".join(lines)


def diff_for(git: Git, change: Change, max_lines: int) -> str:
    match change:
        case Untracked(path):
            out = git.run("diff", "--no-color", "--no-index", "--", os.devnull, path, check=False)
        case _:
            out = git.run("diff", "--no-color", "HEAD", "--", change.path, check=False)
    return _truncate(out, max_lines)


def cmd_collect(args: argparse.Namespace) -> int:
    cfg = load_config(args.config)
    git = resolve_repo(cfg)
    include_untracked = cfg.include_untracked and not args.no_untracked
    max_lines = args.max_diff_lines or cfg.max_diff_lines
    changes = collect_changes(git, include_untracked)
    if not changes:
        print("=== auto-commit: no changes ===")
        return 0
    print(f"=== auto-commit: {len(changes)} changed path(s) ===\n")
    for c in changes:
        print(f"--- [{label_of(c)}] {c.path}{origin_note(c)} ---")
        body = diff_for(git, c, max_lines)
        print(body if body.strip() else "(no textual diff)")
        print()
    return 0


# --- commit -------------------------------------------------------------------


def _body_lines(body: Any) -> tuple[str, ...]:
    match body:
        case str() as s:
            line = s.strip()
            return (line,) if line else ()
        case list() as items:
            return tuple(f"- {s}" for s in (str(b).strip() for b in items) if s)
        case _:
            return ()


def render_message(spec: CommitSpec, trailers: tuple[str, ...]) -> str:
    match spec.scope:
        case str(scope) if scope:
            header = f"{spec.type}({scope}): {spec.subject}"
        case _:
            header = f"{spec.type}: {spec.subject}"
    chunks = [c for c in (header, "\n".join(spec.body), "\n".join(trailers)) if c]
    return "\n\n".join(chunks) + "\n"


def load_plan(spec: str) -> list[Any]:
    match spec:
        case "-":
            raw = sys.stdin.read()
        case _:
            p = Path(spec)
            if not p.is_file():
                die(f"plan file not found: {spec}")
            raw = p.read_text()
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as e:
        die(f"could not parse plan: {e}")
    if not isinstance(data, dict):
        die("plan must be an object with a non-empty `commits` array")
    if not isinstance(commits := data.get("commits"), list) or not commits:
        die("plan must be an object with a non-empty `commits` array")
    return commits


def parse_spec(raw: Any, idx: int, allowed: tuple[str, ...]) -> CommitSpec:
    if not isinstance(raw, dict):
        die(f"commit #{idx} must be a JSON object")
    if not (ctype := str(raw.get("type") or "").strip()):
        die(f"commit #{idx} needs a non-empty `type`")
    if allowed and ctype not in allowed:
        die(f"commit #{idx}: type '{ctype}' not in allowed_types {list(allowed)}")
    if not (subject := str(raw.get("subject") or "").strip()):
        die(f"commit #{idx} needs a non-empty `subject`")
    if not isinstance(files := raw.get("files"), list) or not files:
        die(f"commit #{idx} needs a non-empty `files` array")
    return CommitSpec(
        type=ctype,
        subject=subject,
        files=tuple(str(f) for f in files),
        scope=str(raw.get("scope") or "").strip() or None,
        body=_body_lines(raw.get("body")),
    )


def parse_specs(raw: list[Any], allowed: tuple[str, ...]) -> tuple[CommitSpec, ...]:
    return tuple(parse_spec(item, i, allowed) for i, item in enumerate(raw, 1))


def build_plan(
    specs: tuple[CommitSpec, ...],
    changes: tuple[Change, ...],
    trailers: tuple[str, ...],
) -> Plan:
    """Validate the specs against the live changed set and render each message.
    Pure — no git side effects. Dies on any inconsistency (unknown file, a file
    in two commits), so an accepted plan can never stage something unexpected."""
    by_path = {c.path: c for c in changes}
    seen: dict[str, int] = {}
    planned: list[PlannedCommit] = []
    for idx, spec in enumerate(specs, 1):
        for f in spec.files:
            if f not in by_path:
                die(f"commit #{idx}: file not in current changes: {f}")
            if f in seen:
                die(f"file listed in two commits (#{seen[f]} and #{idx}): {f}")
            seen[f] = idx
        stage = tuple(p for f in spec.files for p in stage_paths(by_path[f]))
        planned.append(PlannedCommit(spec.files, stage, render_message(spec, trailers)))
    uncovered = tuple(sorted(set(by_path) - set(seen)))
    return Plan(tuple(planned), uncovered)


def apply_plan(git: Git, plan: Plan) -> tuple[MadeCommit, ...]:
    """Clean the index, then stage and commit each group in order. Returns the
    short SHA and subject line of each commit made."""
    git.run("reset", "-q")
    made: list[MadeCommit] = []
    for commit in plan.commits:
        git.run("add", "-A", "--", *commit.stage)
        with tempfile.NamedTemporaryFile("w", suffix=".gitmsg", delete=False) as tf:
            tf.write(commit.message)
            msg_path = tf.name
        try:
            git.run("commit", "-F", msg_path)
        finally:
            os.unlink(msg_path)
        sha = git.run("rev-parse", "--short", "HEAD").strip()
        made.append(MadeCommit(sha, commit.message.splitlines()[0]))
    return tuple(made)


def _print_paths(header: str, paths: tuple[str, ...]) -> None:
    if not paths:
        return
    print(header)
    for p in paths:
        print(f"  {p}")


def cmd_commit(args: argparse.Namespace) -> int:
    cfg = load_config(args.config)
    git = resolve_repo(cfg)
    specs = parse_specs(load_plan(args.plan), cfg.allowed_types)
    plan = build_plan(specs, collect_changes(git, cfg.include_untracked), cfg.trailers)

    if args.dry_run:
        print("=== auto-commit: dry run (nothing committed) ===\n")
        for idx, commit in enumerate(plan.commits, 1):
            print(f"--- commit #{idx}: {', '.join(commit.files)} ---")
            print(commit.message)
        _print_paths("--- uncovered (would stay uncommitted) ---", plan.uncovered)
        return 0

    made = apply_plan(git, plan)
    print(f"=== auto-commit: {len(made)} commit(s) created ===")
    for c in made:
        print(f"  {c.sha}  {c.subject}")
    _print_paths(f"\nuncovered, still uncommitted ({len(plan.uncovered)}):", plan.uncovered)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(prog="auto_commit.py", add_help=True)
    ap.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG,
        help="config.json (default: bundled next to the skill)",
    )
    sub = ap.add_subparsers(dest="cmd", required=True)

    c = sub.add_parser("collect", help="print changed paths + diffs")
    c.add_argument(
        "--max-diff-lines",
        type=int,
        default=0,
        help="cap diff lines per file (default: from config)",
    )
    c.add_argument("--no-untracked", action="store_true", help="skip untracked files")

    m = sub.add_parser("commit", help="commit grouped changes from a plan")
    m.add_argument("--plan", required=True, help="plan.json path, or - for stdin")
    m.add_argument(
        "--dry-run", action="store_true", help="render messages and print the plan; commit nothing"
    )

    args = ap.parse_args()
    match args.cmd:
        case "collect":
            return cmd_collect(args)
        case "commit":
            return cmd_commit(args)
        case _:  # unreachable: subparser is required
            ap.error(f"unknown command: {args.cmd}")


if __name__ == "__main__":
    sys.exit(main())
