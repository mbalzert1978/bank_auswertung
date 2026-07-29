---
name: auto-commit
description: Turn an uncommitted working tree into a set of cohesive Conventional-Commits, one per logical change. Reads `git status` and per-file diffs, groups files that belong to the same change (a class and its repository, a function and its test), drafts a templated message per group, shows the plan for confirmation, then commits each group. Use when the user wants to auto-commit, commit their changes, split a messy working tree into clean commits, or asks for grouped/semantic commits with generated messages.
arguments: Optional. A path to scope to, or guidance like "everything as one commit". If omitted, all changes in the current repo are considered.
---

# Auto Commit

Take a working tree full of unrelated changes and turn it into a handful of
**cohesive commits** — one per logical change — each with a Conventional-Commits
message describing *what* was done. A bundled script does the deterministic git
work (status, diffs, staging, committing, rendering the message template); the
**grouping is your judgement** — a script can't tell that `A` and `ARepository`
belong together, but you can by reading the diffs.

The user has chosen: **always show the plan and get confirmation before
committing.** Never commit without that confirmation. Never push.

## Process

1. **Collect.** Run the script to get every changed path with a status label and
   its diff:

   ```bash
   uv run .claude/skills/auto-commit/scripts/auto_commit.py collect
   ```

   The script needs Python 3.10+; it carries PEP 723 metadata so `uv run`
   provisions a suitable interpreter even when the system `python3` is older.
   Read the diffs — they are the only source of truth for what changed. Do not
   invent changes that aren't in the diff.

2. **Group by cohesion.** Put files that implement *one* logical change into the
   same commit; give independent changes their own commit. Signals that two
   files belong together: a type and its repository/DAO/factory, an
   implementation and its test, a model and its migration, a function and the
   call site it was extracted from, a renamed symbol and every file updated for
   it. When unsure, prefer a smaller, more coherent commit over lumping. If the
   user passed guidance (e.g. "all as one commit"), follow it.

3. **Draft a message per group** (Conventional Commits):
   - Header `type(scope): subject` — `type` from the allowed list in
     `config.json`; `scope` optional; `subject` imperative, lower-case, ≤ ~72
     chars ("add A and its repository", not "Added some files").
   - `body` as bullet lines saying *what* changed and *why*, not how. Omit for
     trivial changes.

4. **Write the plan** to the scratchpad as `plan.json` (schema below), one entry
   per group, every changed file you intend to commit assigned to exactly one
   group.

5. **Show the plan and confirm.** Present the numbered commits (files + rendered
   header) and call out anything left uncovered. Get the user's go-ahead; revise
   and re-show if they want changes. You can preview the exact messages without
   committing:

   ```bash
   uv run .claude/skills/auto-commit/scripts/auto_commit.py commit --plan plan.json --dry-run
   ```

6. **Commit** once confirmed:

   ```bash
   uv run .claude/skills/auto-commit/scripts/auto_commit.py commit --plan plan.json
   ```

   The script unstages everything, then stages and commits each group in order.
   Report the resulting short hashes and any files left uncommitted.

## Plan schema

```json
{
  "commits": [
    {
      "type": "feat",
      "scope": "repository",
      "subject": "add A and its repository",
      "body": ["introduce class A", "add ARepository for persistence of A"],
      "files": ["src/A.kt", "src/ARepository.kt"]
    },
    {
      "type": "feat",
      "subject": "add B",
      "body": "introduce class B",
      "files": ["src/B.kt"]
    }
  ]
}
```

`scope` and `body` are optional; `body` may be a string or a list of lines.
Configured `trailers` (e.g. `Co-Authored-By`) are appended by the script — don't
put them in the body. The script **rejects** the plan if a file isn't currently
changed, if a file appears in two groups, or if a `type` is outside
`allowed_types` — so the plan can never commit something unexpected.

## Config

`config.json` (bundled next to the skill) holds the knobs:

- `repo` — absolute path of the repository to operate on. Leave empty (default)
  to auto-detect the work-tree root from the current directory; set it to pin
  every run to one repo regardless of where the skill is invoked. All git
  commands run with `git -C <repo>`, so `collect` works even when invoked from
  the skill's own folder.
- `include_untracked` — stage new/untracked files too (default `true`).
- `max_diff_lines` — per-file diff cap in `collect` output (default `400`).
- `allowed_types` — the Conventional-Commit types a message may use.
- `trailers` — lines appended to every commit message; empty the array to drop
  the `Co-Authored-By` trailer.

## Rules

- **Confirm before committing. Never push.** Committing only.
- One logical change per commit; never lump unrelated changes to save a commit.
- Cover only what the diff shows; don't describe changes that aren't there.
- Files not assigned to any group stay uncommitted — report them, don't
  force-commit them to "use everything up".
- If the working tree is clean, say so and stop.

## Report format

After committing, a short table — skip it if there was nothing to commit:

| # | Commit | Files |
|---|--------|-------|
| 1 | `feat(repository): add A and its repository` | `A.kt`, `ARepository.kt` |
| 2 | `feat: add B` | `B.kt` |

End with anything left uncommitted, or "working tree clean" if nothing remains.
