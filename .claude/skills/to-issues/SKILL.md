---
name: to-issues
description: Break a plan, spec, or PRD into independently-grabbable issues as markdown files under docs/issues/, using tracer-bullet vertical slices, gating the breakdown against an objective PASS/FAIL check before publishing. Use when user wants to convert a plan into issues, create implementation tickets, or break down work into issues.
arguments: Optional. A reference to the source to break down — an issue ID (e.g. 0021), a URL, or a path to a plan/PRD file. If omitted, the skill works from whatever is already in the conversation context.
---

# To Issues

Break a plan into independently-grabbable issues using vertical slices (tracer bullets).

## Process

### 1. Gather context

Work from whatever is already in the conversation context. If the user passes an issue reference as an argument:

- **ID/slug** (e.g. `0021`, or a partial filename) → resolve it against `docs/issues/` the same way `issue-close` does: match files whose name starts with the given string, excluding `PROGRESS.md`. Read the matched file's full body.
- **URL or path to external material** (e.g. a linked plan/PRD outside `docs/issues/`) → fetch and read it directly.

### 2. Explore the codebase (optional)

If you have not already explored the codebase, do so to understand the current state of the code. Issue titles and descriptions should use the project's domain glossary vocabulary, and respect ADRs in the area you're touching.

### 3. Draft vertical slices

Break the plan into **tracer bullet** issues. Each issue is a thin vertical slice that cuts through ALL integration layers end-to-end, NOT a horizontal slice of one layer.

Slices may be 'HITL' or 'AFK'. HITL slices require human interaction, such as an architectural decision or a design review. AFK slices can be implemented and merged without human interaction. Prefer AFK over HITL where possible.

<vertical-slice-rules>
- Each slice delivers a narrow but COMPLETE path through every layer (schema, API, UI, tests)
- A completed slice is demoable or verifiable on its own
- Prefer many thin slices over few thick ones
</vertical-slice-rules>

### 4. Quiz the user

Present the proposed breakdown as a numbered list. For each slice, show:

- **Title**: short descriptive name
- **Type**: HITL / AFK
- **Blocked by**: which other slices (if any) must complete first
- **User stories covered**: which user stories this addresses (if the source material has them)

Ask the user:

- Does the granularity feel right? (too coarse / too fine)
- Are the dependency relationships correct?
- Should any slices be merged or split further?
- Are the correct slices marked as HITL and AFK?

Iterate until the user approves the breakdown.

### 5. Gate the breakdown (PASS/FAIL)

Before publishing, run the `verify-issue-breakdown` skill on the approved breakdown. It returns an objective PASS/FAIL over plan coverage, vertical slices, dependency-graph soundness, independent verifiability, granularity, and template/hygiene.

- **PASS** → proceed to publish.
- **FAIL** → revise the offending slices (split horizontals, fill coverage gaps, break cycles, add acceptance criteria) and re-run the gate. Cap at `max_gate_attempts` from `config.json` (default 3), then surface the remaining failures to the user. **Never publish on a FAIL.**

### 6. Write the issues to `docs/issues/`

For each approved slice, in dependency order (blockers first), write it as its own file under `docs/issues/` — no external issue tracker is involved.

**Assign the ID, filename, and status — and bootstrap `PROGRESS.md`.** Both are deterministic, so run the bundled script rather than doing either by hand — one call per slice, in dependency order:

```bash
uv run .claude/skills/to-issues/scripts/next_issue.py --title "<slice title>" [--blocked-by ID [ID ...]]
```

On its first call it also creates `docs/issues/PROGRESS.md` from [assets/progress-template.md](assets/progress-template.md) if missing — `issue-close` requires that file (a `## Erledigt` heading + a `Stand:` line). Pass `--blocked-by` with the real ID(s) already assigned to this slice's blockers (from this run, or a pre-existing issue). It prints:

```json
{ "id": "NNNN", "filename": "NNNN-kebab-case-title.md", "status": "open|blocked", "progress_bootstrapped": true|false }
```

- `id` — the next unused 4-digit ID under `docs/issues/`, zero-padded (`0001` if the directory is empty or doesn't exist yet).
- `filename` — `<id>` + a slug of `--title`.
- `status` — `"open"` if no `--blocked-by` was given, or every given blocker's **own current frontmatter `status`** is already `"closed"`; `"blocked"` otherwise. (This checks each blocker's actual status, not just whether it was freshly written — so a slice blocked only by an already-closed issue from an earlier run still comes out `open`.)
- `progress_bootstrapped` — `true` only on the run that just created `PROGRESS.md`; informational, nothing else to do with it.

It exits with an error if a `--blocked-by` ID doesn't resolve to a file — that means dependency order was violated; publish that blocker first.

**Compose the file.** Read [assets/issue-template.md](assets/issue-template.md) and fill it in to compose the body — don't regenerate the scaffold inline. Prepend YAML frontmatter with the `id`/`status` from the script plus the slice `title`, and a title heading:

```markdown
---
id: "NNNN"
title: <slice title>
status: open|blocked
---

# <slice title>

<filled assets/issue-template.md body>
```

In the "Blocked by" section, reference each blocker by its real `id`/`filename`, e.g. `Blocked by [0021](0021-materialise-sibling-sizes-at-group-formation.md)`. These issues are considered ready for AFK agents; `status: open` is the ready signal, so no external triage label is needed.

Write the file to `docs/issues/<filename>` using the script's output.

Do NOT close or modify any pre-existing issue file (e.g. a parent issue loaded in step 1).
