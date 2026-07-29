---
name: worktree-entfernen
description: "Tears down a git worktree under .claude/worktrees/ — but only when it is final in main (its branch fully merged into the main branch) and its working tree is clean. Removes the worktree junction-safely and deletes the merged branch. Use when the user wants to 'remove a worktree', 'einen Worktree entfernen/abbauen', 'clean up the worktree for branch X', or tear down a worktree after its PR merged."
arguments: "Worktree name (the directory under .claude/worktrees/). Append 'force' to override the safety gate and remove an unclean or unmerged worktree anyway (may lose work). Append 'keep-branch' to retain the branch even when it was merged. If the name is omitted, ask for it before proceeding."
---

# Worktree entfernen

Tear down a git worktree created under `.claude/worktrees/` — safely. The whole point
of this skill is the **safety gate**: a worktree is only removable once its work is
*final in main*.

## The safety (non-negotiable by default)

A worktree is removed only when **both** hold:

1. **Final in main** — every commit on its branch is reachable from the configured
   `main_branch`. If the branch still has commits that main doesn't, the worktree is
   *not* final and removal is **refused**.
2. **Clean working tree** — no uncommitted or untracked changes in the worktree.

Either condition unmet is a **blocker**; the script names it and exits non-zero without
touching anything. `--force` (argument `force`) is the explicit, loud escape hatch for
deliberately abandoning unfinished work — it never fires by accident, prints a warning,
and an abandoned **unmerged branch is always retained** so its commits are never silently
lost.

> **Note on squash/rebase merges:** "final in main" is reachability-based (`git rev-list`).
> A branch merged via **squash or rebase** has commits that aren't literally reachable from
> main, so it will read as *not final* and need `force`. This is the safe default — it errs
> toward keeping, never toward silent loss.

## config.json

```json
{
  "worktree_base": ".claude/worktrees",
  "main_branch": "main"
}
```

- `worktree_base` — where worktrees live (must match `worktree-erstellen`).
- `main_branch` — the branch a worktree must be merged into to count as final. Must exist locally; the merge check compares against your **local** main, so pull it first for an accurate verdict.

## Junction-safe teardown

This repo's worktrees contain junctions/symlinks into the main checkout's `.claude/`
(created by `worktree-erstellen`). Teardown must never follow them:

- `git worktree remove --force` unregisters the worktree and (correctly) will **not**
  recurse into the junctions — so the main `.claude/` is never touched.
- On **Windows** that leaves the now-empty worktree directory on disk; it is removed with
  `rmdir /s /q`, which deletes junction reparse points **without** following them. On
  **POSIX**, `shutil.rmtree` unlinks symlinks without traversing them.

## Process

### 1. Parse `arguments`
- First token → **name** (the directory under `worktree_base`). If absent, ask for it and stop.
- `force` token → pass `--force` (override the safety gate).
- `keep-branch` token → pass `--keep-branch` (retain a merged branch).

### 2. Run the bundled script from the repo root
```powershell
uv run .claude\skills\worktree-entfernen\scripts\teardown_worktree.py <name> [--force] [--keep-branch]
```

The script resolves the repo root; refuses non-worktrees, the primary checkout, and a
worktree sitting on `main_branch`; then runs the safety gate. If clean and final (or
`--force`), it removes the worktree junction-safely, prunes, and deletes the merged branch
(unless `--keep-branch`). It **never** partially removes: a blocked run changes nothing.

### 3. Report
Relay the script's action log verbatim, or — on a refusal — the exact blocker(s) so the
user knows what to commit/merge before retrying.

## Report format

On success, one line per action:

```
removed   C:\...\.claude\worktrees\0031-fix-parser  (worktree unregistered)
cleaned   C:\...\.claude\worktrees\0031-fix-parser  (leftover directory removed (junction-safe))
deleted   0031-fix-parser  (branch (merged into main))
```

Status words: `removed` (worktree unregistered), `cleaned` (leftover dir removed),
`deleted` (merged branch removed), `kept` (branch retained).

On a refusal, the script exits 1 with, e.g.:

```
worktree-entfernen: error: refusing to remove '0031-fix-parser' — branch '0031-fix-parser'
has 3 commit(s) not in 'main' — not final in main.
  Pass --force to override (may lose work).
```

## Done criterion

- A clean, merged worktree: unregistered, its directory gone, its branch deleted (unless `--keep-branch`), exit 0.
- A dirty or unmerged worktree without `--force`: **nothing changed**, exit 1, blocker named.
- The main checkout's `.claude/` is never touched (junctions are removed, never followed).
