"""Shared git helpers: resolve a base branch and list paths changed vs. it.

default_base() tries origin/HEAD, then origin/main, origin/master, main,
master in that order. resolve_merge_base() is the shared "am I in a work
tree, can I resolve a base, is there a merge-base" preflight; changed_paths()
is the actual diff, split out because callers that also need the merge_base
value itself (e.g. for `git show <merge_base>:<file>`) call both, while
callers that only need the file list call changed_paths() alone.
"""

import subprocess
from dataclasses import dataclass


def git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", *args], capture_output=True, text=True)


def default_base() -> str | None:
    head = git("rev-parse", "--abbrev-ref", "origin/HEAD")
    if head.returncode == 0 and head.stdout.strip():
        return head.stdout.strip()
    for cand in ("origin/main", "origin/master", "main", "master"):
        if git("rev-parse", "--verify", "--quiet", cand).returncode == 0:
            return cand
    return None


@dataclass(frozen=True)
class GitError:
    message: str


def resolve_merge_base(base: str | None) -> str | GitError:
    if git("rev-parse", "--is-inside-work-tree").returncode != 0:
        return GitError("not inside a git work tree")
    base = base or default_base()
    if base is None:
        return GitError("could not determine a base branch (pass one explicitly)")
    merge_base = git("merge-base", "HEAD", base).stdout.strip()
    if not merge_base:
        return GitError(f"no merge-base between HEAD and {base}")
    return merge_base


def changed_paths(merge_base: str) -> tuple[str, ...]:
    names = git("diff", "--name-only", "--diff-filter=d", merge_base)
    return tuple(n for n in names.stdout.splitlines() if n.strip())
