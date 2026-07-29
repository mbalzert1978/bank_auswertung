#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""worktree-entfernen: tear down a git worktree — but only when it is final in main.

Requires Python 3.10+ (match statements, PEP 604 `X | Y` unions). The PEP 723
metadata block lets `uv run` provision a suitable interpreter regardless of the
system default — invoke via `uv run scripts/teardown_worktree.py …` (see SKILL.md).

The safety, non-negotiable by default: a worktree is only removable when it is
*final in main* — every commit on its branch is reachable from the configured main
branch — AND its working tree is clean. Either condition unmet is a blocker; the
script refuses and names it. `--force` is the explicit, loud escape hatch for
abandoning unfinished work (may lose commits/changes); it never fires by accident.

Teardown is junction-safe. On Windows `git worktree remove` unregisters the worktree
but leaves the directory behind (it correctly won't recurse into the junctions this
repo's worktrees use), so the leftover is removed with `rmdir /s /q`, which deletes
junction reparse points without following them into the main checkout. A merged
branch is deleted too (opt out with --keep-branch); an abandoned unmerged branch is
always retained so its commits are never silently lost.

Usage:
  teardown_worktree.py <name> [--force] [--keep-branch] [--config cfg.json]

Exit code is 0 on success, 1 on a usage/config/git/safety error.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass, fields
from pathlib import Path
from typing import NoReturn, TypeAlias

HERE = Path(__file__).resolve().parent
DEFAULT_CONFIG = HERE.parent / "config.json"


def die(msg: str) -> NoReturn:
    print(f"worktree-entfernen: error: {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd: list[str]) -> subprocess.CompletedProcess:
    # Force UTF-8: git/rmdir output may carry bytes the OEM/ANSI codepage can't decode.
    return subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")


# --- Domain model: immutable records, illegal states unrepresentable ----------


@dataclass(frozen=True)
class Git:
    """The git CLI bound to one repo root; every call runs `git -C root …`, so it
    behaves the same no matter what directory the script is invoked from."""

    root: str

    def run(self, *args: str, check: bool = True) -> str:
        res = run(["git", "-C", self.root, *args])
        if check and res.returncode != 0:
            die(f"`git {' '.join(args)}` failed:\n{res.stderr.strip()}")
        return res.stdout

    def branch_exists(self, name: str) -> bool:
        return bool(self.run("show-ref", "--verify", f"refs/heads/{name}", check=False).strip())

    def records(self) -> list[dict[str, str]]:
        """`git worktree list --porcelain` as one dict per registered worktree."""
        out: list[dict[str, str]] = []
        for block in self.run("worktree", "list", "--porcelain").split("\n\n"):
            rec = dict(line.partition(" ")[::2] for line in block.splitlines() if line)
            if "worktree" in rec:
                out.append(rec)
        return out

    def primary(self) -> Path:
        """The main working tree — always the first entry of `worktree list`."""
        return Path(self.records()[0]["worktree"]).resolve()

    def branch_at(self, path: Path) -> str | None:
        """The branch checked out at `path`, or None for a detached HEAD / unknown."""
        for rec in self.records():
            if Path(rec["worktree"]).resolve() == path.resolve():
                ref = rec.get("branch", "")
                return ref[len("refs/heads/") :] if ref.startswith("refs/heads/") else None
        return None

    def commits_ahead(self, branch: str, base: str) -> int:
        return int(self.run("rev-list", "--count", branch, f"^{base}").strip() or "0")


@dataclass(frozen=True)
class Config:
    """The parsed config.json — validated once at the boundary, then trusted."""

    worktree_base: str
    main_branch: str = "main"


@dataclass(frozen=True)
class Dirty:
    """The worktree has uncommitted or untracked changes."""

    changes: tuple[str, ...]


@dataclass(frozen=True)
class Unmerged:
    """The branch has commits not yet reachable from the main branch."""

    ahead: int


# A safety blocker is one of these; several can hold at once, so they are collected
# as a tuple rather than collapsed into a single verdict.
Blocker: TypeAlias = Dirty | Unmerged


def describe(blocker: Blocker, branch: str | None, main: str) -> str:
    match blocker:
        case Dirty(changes):
            return f"working tree not clean ({len(changes)} uncommitted change(s)) - commit or stash first"
        case Unmerged(ahead):
            return f"branch '{branch}' has {ahead} commit(s) not in '{main}' - not final in main"


# --- Safety assessment: pure over git queries ---------------------------------


def safety_blockers(git: Git, worktree: Path, branch: str | None, main: str) -> tuple[Blocker, ...]:
    blockers: list[Blocker] = []
    changes = Git(str(worktree)).run("status", "--porcelain", "-z").split("\0")
    if dirty := tuple(c for c in changes if c):
        blockers.append(Dirty(dirty))
    if branch is not None and (ahead := git.commits_ahead(branch, main)) > 0:
        blockers.append(Unmerged(ahead))
    return tuple(blockers)


# --- Removal mechanics: never follow a reparse point into its source ----------


def remove_tree(path: Path) -> None:
    """Recursively delete `path` WITHOUT following junctions/symlinks into their
    target — otherwise we would delete the main checkout's linked .claude content."""
    if os.name == "nt":
        run(["cmd", "/c", "rmdir", "/s", "/q", str(path)])  # rmdir removes junctions, never follows
    else:
        shutil.rmtree(path)  # rmtree unlinks symlinks, does not traverse them


def fmt(status: str, name: str, detail: str) -> str:
    """The single log-line shape shared by every action."""
    return f"{status:<9} {name}  ({detail})"


def teardown(git: Git, worktree: Path, branch: str | None, delete_branch: bool) -> list[str]:
    log = [fmt("removed", str(worktree), "worktree unregistered")]
    git.run("worktree", "remove", "--force", str(worktree))
    if worktree.exists():  # Windows: git leaves the junction-holding dir behind
        remove_tree(worktree)
        log.append(fmt("cleaned", str(worktree), "leftover directory removed (junction-safe)"))
    git.run("worktree", "prune")
    if delete_branch and branch is not None:
        git.run("branch", "-d", branch)  # -d refuses an unmerged branch: belt-and-braces
        log.append(fmt("deleted", branch, "branch (merged into main)"))
    elif branch is not None:
        log.append(fmt("kept", branch, "branch retained"))
    return log


# --- Config & repo resolution -------------------------------------------------


def load_config(path: Path) -> Config:
    if not path.is_file():
        die(f"config file not found: {path}")
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        die(f"could not parse config {path}: {e}")
    if not isinstance(raw, dict):
        die(f"config {path} must be a JSON object")
    if unknown := set(raw) - {f.name for f in fields(Config)}:
        die(f"unknown config keys in {path}: {', '.join(sorted(unknown))}")
    if not (base := str(raw.get("worktree_base") or "").strip()):
        die(f"config {path} needs a non-empty `worktree_base`")
    return Config(worktree_base=base, main_branch=str(raw.get("main_branch") or "main").strip())


def resolve_repo() -> Git:
    res = run(["git", "rev-parse", "--show-toplevel"])
    if res.returncode != 0:
        die("not inside a git repository")
    return Git(res.stdout.strip())


# --- main ---------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser(prog="teardown_worktree.py", description=__doc__)
    ap.add_argument("name", help="worktree directory name under worktree_base")
    ap.add_argument("--force", action="store_true", help="override the safety gate (may lose work)")
    ap.add_argument("--keep-branch", action="store_true", help="keep the branch even when merged")
    ap.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="config.json (default: bundled)")
    args = ap.parse_args()

    cfg = load_config(args.config)
    git = resolve_repo()
    worktree = (Path(git.root) / cfg.worktree_base / args.name).resolve()

    # Preconditions — usage errors, independent of the safety gate.
    if worktree not in {Path(r["worktree"]).resolve() for r in git.records()}:
        die(f"not a registered worktree: {worktree}")
    if worktree == git.primary():
        die("refusing to remove the primary worktree (the main checkout)")
    branch = git.branch_at(worktree)
    if branch == cfg.main_branch:
        die(f"refusing to remove a worktree checked out on '{cfg.main_branch}'")
    if branch is None and not args.force:
        die("worktree is in detached HEAD — cannot verify it is final in main; pass --force to remove anyway")
    if branch is not None and not git.branch_exists(cfg.main_branch):
        die(f"main branch '{cfg.main_branch}' not found locally — cannot verify merge state")

    # Safety gate — the reason this skill exists.
    blockers = safety_blockers(git, worktree, branch, cfg.main_branch)
    if blockers:
        reasons = "; ".join(describe(b, branch, cfg.main_branch) for b in blockers)
        if not args.force:
            die(f"refusing to remove '{args.name}' - {reasons}.\n  Pass --force to override (may lose work).")
        print(f"worktree-entfernen: WARNING: forced removal despite: {reasons}", file=sys.stderr)

    merged = not any(isinstance(b, Unmerged) for b in blockers)
    delete_branch = merged and not args.keep_branch
    print("\n".join(teardown(git, worktree, branch, delete_branch)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
