---
name: worktree-erstellen
description: "Creates a git worktree under .claude/worktrees/ and makes the local-only project context (.claude/, CLAUDE.md, CONTEXT.md) available inside it, so an agent started in the worktree sees the same skills, agents, settings, and docs as the main checkout. Use when the user wants to 'create a worktree', 'spin up a worktree for branch X', 'einen Worktree anlegen', or complains that an agent in a worktree can't see .claude/ / the project skills / CLAUDE.md."
arguments: "Worktree name, optionally followed by a branch — e.g. '0031-fix-parser' or '0031-fix-parser feature/parser'. The name becomes the directory under .claude/worktrees/. If no branch is given it defaults to the name (existing branch is checked out, otherwise a new one is created from HEAD). If the name is omitted, ask for it before proceeding. Append 'refresh' to re-create every link/copy."
---

# Worktree erstellen

Create a git worktree and mirror the repo's **local-only context** into it. A fresh
worktree receives only *tracked* files; this repo gitignores almost all of `.claude/`
(`.claude/*`, keeping only `settings.json` + `hooks/`), so a worktree agent otherwise
can't see the project's skills, agents, commands, local settings, or memory. This skill
closes that gap in one idempotent step.

## config.json

```json
{
  "worktree_base": ".claude/worktrees",
  "targets": ["CLAUDE.md", "CONTEXT.md"],
  "expand_dirs": [".claude"],
  "expand_skip": ["worktrees"]
}
```

- `worktree_base` — where worktrees live (the repo convention: local-only, gitignored). Do not change without updating `.gitignore`.
- `targets` — explicit context paths to make available (tracked → git provides them; a link is only created if a genuine gap exists).
- `expand_dirs` — directories whose **children** are handled individually: gitignored children (`skills/`, `agents/`, `commands/`, `settings.local.json`, `projects/`) get linked; tracked children (`settings.json`, `hooks/`) are left to the checkout.
- `expand_skip` — child names never touched. `worktrees` is skipped so a worktree never links its own parent (no recursion).

## How context is made available

The script picks the method by platform and target kind — **no hardcoded path separator** (pathlib):

| Platform | Directory                | File                          |
|----------|--------------------------|-------------------------------|
| POSIX    | relative symlink (`ln -s`) | relative symlink              |
| Windows  | junction (`mklink /J`, no admin) | hardlink (`os.link`, no admin) |
| Fallback | copy                     | copy                          |

**Trade-off — link vs. copy:** a symlink/junction/hardlink *follows the source*, so an
edit to `.claude/skills/…` in the main checkout is instantly visible in the worktree. A
**copy is a point-in-time snapshot that drifts** — it is only used when the platform
primitive fails, and is refreshed only by re-running with `refresh`. Windows junctions use
an **absolute** target (they break if the repo is moved); POSIX symlinks are relative and
survive a move; hardlinks are same-volume only. `CLAUDE.md`/`CONTEXT.md` are tracked, so
git already places them in the worktree — the script reports them as checkout-provided and
does not touch them unless they are genuinely missing.

## Process

### 1. Parse `arguments`
- First token → **name** (the directory under `worktree_base`). If absent, ask for it and stop.
- Second token (if not `refresh`) → **branch**. Omitted → branch defaults to the name.
- A `refresh` token anywhere → pass `--refresh`.

### 2. Run the bundled script from the repo root
```powershell
uv run .claude\skills\worktree-erstellen\scripts\setup_worktree.py <name> [branch] [--refresh]
```

The script: resolves the repo root; creates the worktree via `git worktree add` (checks
out an existing branch, else creates one from HEAD with `-b`); then, for every context
target, creates or repairs the platform-appropriate link. It is **idempotent** — an
already-correct link is left alone, a missing or broken one is repaired, and re-running on
an existing worktree never fails.

### 3. Report
Relay the script's action log to the user (see below), then state the worktree path so the
user can `cd` into it or point an agent at it.

## Report format

The script prints one line per action; surface it verbatim, e.g.:

```
created   .claude/worktrees/0031-fix-parser  (created branch '0031-fix-parser')
git       CLAUDE.md  (provided by checkout)
git       CONTEXT.md  (provided by checkout)
created   .claude/agents  (junction)
created   .claude/commands  (junction)
created   .claude/settings.local.json  (hardlink)
created   .claude/skills  (junction)
created   .claude/projects  (junction)
```

Status words: `created` (new), `ok` (already correct), `repaired` (broken/wrong link
rebuilt), `refreshed` (rebuilt under `refresh`), `git` (left to the checkout).

## Removing a worktree

Removal is out of scope for this skill, but note the Windows gotcha: `git worktree remove`
unregisters the worktree and (correctly) will **not** recurse into the junctions, so the
main `.claude/` is never touched — but it also leaves the now-empty worktree directory on
disk. Delete that leftover with `rmdir /s /q` (which removes junction reparse points
without following them), never a tool that dereferences junctions. On POSIX, `git worktree
remove` deletes everything cleanly because it unlinks symlinks without following them.

## Done criterion

- The worktree exists under `.claude/worktrees/<name>` on the intended branch.
- Inside it, `.claude/skills`, `.claude/agents`, `.claude/commands`, `.claude/settings.local.json`, and `.claude/projects` resolve to the main checkout's copies (or a copy fallback), and `CLAUDE.md`/`CONTEXT.md` are present.
- Re-running the same invocation reports `ok`/`git` for every target and exits 0.
