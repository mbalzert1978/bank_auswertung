#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""worktree-erstellen: create a git worktree and mirror the local-only context in.

Requires Python 3.10+ (match statements, PEP 604 `X | Y` unions). The PEP 723
metadata block lets `uv run` provision a suitable interpreter regardless of the
system default — invoke via `uv run scripts/setup_worktree.py …` (see SKILL.md).

A fresh `git worktree` only receives *tracked* files. Here `.gitignore` excludes
almost all of `.claude/` (`.claude/*`, negating only `settings.json` + `hooks/`),
so an agent started in a new worktree cannot see the project's skills, agents,
commands, local settings, or memory. This script creates the worktree, then makes
each configured context artifact available inside it — picking the method by
platform, with no hardcoded path separator (pathlib):

  * POSIX  : relative symlink (`os.symlink`) — follows the source, survives a repo move.
  * Windows: directory junction (`mklink /J`) for dirs, hardlink (`os.link`) for files —
             both follow the source, no admin/developer-mode. Junction target is
             absolute (breaks if the repo moves); hardlink is same-volume only.
             If the platform primitive fails, fall back to a plain copy.

A link follows source changes automatically; a copy is a point-in-time snapshot that
drifts and is only refreshed by re-running with --refresh. Idempotent: an already
correct link is left alone, a missing or dangling one is repaired.

Usage:
  setup_worktree.py <name> [branch] [--refresh] [--config cfg.json]

Exit code is 0 on success, 1 on a usage/config/git error.
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
    print(f"worktree-erstellen: error: {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd: list[str]) -> subprocess.CompletedProcess:
    # Force UTF-8: git/mklink output may carry bytes the OEM/ANSI codepage can't decode.
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

    def ignores(self, rel: str) -> bool:
        """True when git would NOT populate `rel` in a checkout (it is gitignored)."""
        return bool(self.run("check-ignore", rel, check=False).strip())

    def worktrees(self) -> set[Path]:
        lines = self.run("worktree", "list", "--porcelain").splitlines()
        return {
            Path(line[len("worktree ") :]).resolve()
            for line in lines
            if line.startswith("worktree ")
        }


@dataclass(frozen=True)
class Config:
    """The parsed config.json — validated once at the boundary, then trusted."""

    worktree_base: str
    targets: tuple[str, ...] = ()
    expand_dirs: tuple[str, ...] = ()
    expand_skip: frozenset[str] = frozenset()


@dataclass(frozen=True)
class Missing:
    """Nothing occupies the link path — not even a dangling reparse point."""


@dataclass(frozen=True)
class ReparseLink:
    """A symlink or junction. `on_target` is False for a dangling or mis-aimed one
    (the repair case: e.g. an absolute junction left over after the repo moved)."""

    on_target: bool


@dataclass(frozen=True)
class RealEntry:
    """A concrete file or directory: a hardlink, a copy, or git-checkout content."""

    hardlink_of_target: bool


# The current occupant of a link path is exactly one of these three shapes.
Occupant: TypeAlias = Missing | ReparseLink | RealEntry


# --- Link mechanics: platform-appropriate primitives, no hardcoded separator ---


def is_reparse(p: Path) -> bool:
    """True for a symlink (any OS) or a Windows junction — even a dangling one."""
    if p.is_symlink():
        return True
    isjunction = getattr(os.path, "isjunction", None)
    try:
        return bool(isjunction and isjunction(p))
    except OSError:
        return False


def classify(link: Path, target: Path) -> Occupant:
    """Pure: what currently sits at `link`, relative to the intended `target`."""
    if is_reparse(link):
        try:
            return ReparseLink(link.resolve() == target.resolve())
        except OSError:
            return ReparseLink(False)  # dangling: resolve() raised
    if link.exists():
        try:
            return RealEntry(not link.is_dir() and os.path.samefile(link, target))
        except OSError:
            return RealEntry(False)
    return Missing()


def create_link(link: Path, target: Path) -> str:
    """Create the platform-appropriate link (or copy fallback). Returns the method."""
    link.parent.mkdir(parents=True, exist_ok=True)
    if os.name == "posix":
        os.symlink(os.path.relpath(target, link.parent), link, target_is_directory=target.is_dir())
        return "symlink"
    if target.is_dir():  # Windows directory → junction, no admin
        if run(["cmd", "/c", "mklink", "/J", str(link), str(target)]).returncode == 0:
            return "junction"
        shutil.copytree(target, link)
        return "copy"
    try:  # Windows file → hardlink, no admin, same volume
        os.link(target, link)
        return "hardlink"
    except OSError:
        shutil.copy2(target, link)
        return "copy"


def remove_link(p: Path) -> None:
    """Remove the link/entry itself — never follow a reparse point into its source."""
    if is_reparse(p):
        os.rmdir(p) if p.is_dir() and not p.is_symlink() else p.unlink()
    elif p.is_dir():
        shutil.rmtree(p)
    else:
        p.unlink()


def ensure(link: Path, target: Path, refresh: bool) -> tuple[str, str]:
    """Idempotently make `link` resolve to `target`. Returns (status, detail)."""
    match classify(link, target):
        case Missing():
            return "created", create_link(link, target)
        case ReparseLink(on_target=True) if not refresh:
            return "ok", "already linked"
        case ReparseLink(on_target=True):
            remove_link(link)
            return "refreshed", create_link(link, target)
        case ReparseLink():  # dangling or aimed elsewhere → repair
            remove_link(link)
            return "repaired", create_link(link, target)
        case RealEntry() if refresh:
            remove_link(link)
            return "refreshed", create_link(link, target)
        case RealEntry(hardlink_of_target=True):
            return "ok", "already linked (hardlink)"
        case RealEntry():
            return "ok", "present (copy or git-provided)"


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
    return Config(
        worktree_base=base,
        targets=tuple(str(t) for t in raw.get("targets", ())),
        expand_dirs=tuple(str(d) for d in raw.get("expand_dirs", ())),
        expand_skip=frozenset(str(s) for s in raw.get("expand_skip", ())),
    )


def resolve_repo() -> Git:
    res = run(["git", "rev-parse", "--show-toplevel"])
    if res.returncode != 0:
        die("not inside a git repository")
    return Git(res.stdout.strip())


# --- worktree + context provisioning ------------------------------------------


def ensure_worktree(git: Git, path: Path, branch: str) -> tuple[str, str]:
    """Create the worktree if absent. Returns (status, detail)."""
    if path.resolve() in git.worktrees():
        return "ok", "worktree already exists"
    if git.branch_exists(branch):
        git.run("worktree", "add", str(path), branch)
        return "created", f"checked out existing branch '{branch}'"
    git.run("worktree", "add", "-b", branch, str(path))
    return "created", f"created branch '{branch}'"


def context_targets(root: Path, cfg: Config) -> list[str]:
    """Expand config into concrete relative paths to mirror into the worktree:
    the explicit `targets`, plus each gitignore-decided child of `expand_dirs`."""
    targets = list(cfg.targets)
    for d in cfg.expand_dirs:
        src = root / d
        if not src.is_dir():
            continue
        for child in sorted(src.iterdir(), key=lambda p: p.name):
            if child.name not in cfg.expand_skip:
                targets.append(f"{d}/{child.name}")
    return targets


def fmt(status: str, name: str, detail: str) -> str:
    """The single log-line shape shared by every action."""
    return f"{status:<9} {name}  ({detail})"


# --- main ---------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser(prog="setup_worktree.py", description=__doc__)
    ap.add_argument("name", help="worktree directory name (placed under worktree_base)")
    ap.add_argument("branch", nargs="?", help="branch to check out or create (default: name)")
    ap.add_argument("--refresh", action="store_true", help="re-create every link/copy even if present")
    ap.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="config.json (default: bundled)")
    args = ap.parse_args()

    cfg = load_config(args.config)
    git = resolve_repo()
    root = Path(git.root)
    branch = args.branch or args.name
    worktree = (root / cfg.worktree_base / args.name).resolve()

    wt_status, wt_detail = ensure_worktree(git, worktree, branch)
    log = [fmt(wt_status, str(worktree), wt_detail)]

    # The worktree's own .claude/ (tracked settings.json + hooks/) must exist as a
    # real dir before we hang gitignored siblings off it.
    (worktree / ".claude").mkdir(parents=True, exist_ok=True)

    for rel in context_targets(root, cfg):
        target = root / rel
        if not target.exists():
            continue
        link = worktree / rel
        if not git.ignores(rel) and link.exists():
            # git populates this path in the checkout; only fill a genuine gap.
            log.append(fmt("git", rel, "provided by checkout"))
            continue
        status, detail = ensure(link, target, args.refresh)
        log.append(fmt(status, rel, detail))

    print("\n".join(log))
    return 0


if __name__ == "__main__":
    sys.exit(main())
