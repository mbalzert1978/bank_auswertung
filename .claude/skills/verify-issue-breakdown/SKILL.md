---
name: verify-issue-breakdown
description: Verify that a to-issues breakdown is a sound tracer-bullet decomposition before it is published — full plan coverage, every slice vertical, an acyclic dependency graph, and each issue on-template — and return PASS/FAIL. Use when checking a drafted issue breakdown, verifying vertical slices before publishing to the tracker, or gating a to-issues run before publish.
arguments: "Primary: the drafted slice list + its source plan/PRD to verify — a path, a URL, or inline text (default: read both from the conversation context, the to-issues breakdown). Optional `glossary_path`: path to the domain glossary / ADRs to enable criterion 6's vocabulary sub-check (default: discover in the repo, else skip that sub-check)."
---

# Verify Issue Breakdown

Verify that a drafted `to-issues` breakdown is a sound tracer-bullet decomposition of its source plan, and return an objective verdict: **PASS / FAIL**. This is a pre-publish gate — run it on the proposed slice list, before any issue hits the tracker.

## Verdict

**PASS** when *every* criterion below passes.

**FAIL** when *any* criterion fails. The first three are non-negotiable (a coverage gap, a horizontal slice, or a broken dependency graph means the breakdown is wrong, not just rough). The last three fail only on the clear violations defined in each criterion — when in doubt, pass and note the concern rather than blocking.

## Criteria

Each criterion is testable and feeds the verdict.

1. **Plan coverage** — every requirement / user story in the source plan maps to at least one slice, and no slice introduces scope absent from the plan. Build a requirement→slice map; FAIL on any unmapped requirement or any orphan slice. — correctness
2. **Vertical slices** — each slice's "What to build" describes end-to-end behavior crossing every relevant layer (schema, API, UI, tests as applicable), not a single layer. FAIL if any slice is horizontal (e.g. "add the DB column", "build the endpoint" with no path through to a verifiable outcome). — correctness
3. **Sound dependency graph** — the "Blocked by" edges form a DAG: every referenced blocker exists in the set, there are no cycles, and no dangling references. Checked by `scripts/check_dag.py` (see *Process*) — don't re-derive the graph logic by hand. FAIL on any cycle or missing referent. — correctness
4. **Independently verifiable** — each slice is demoable / verifiable on its own and carries concrete, checkable acceptance criteria (not restatements of the title). FAIL if any slice lacks testable acceptance criteria. — quality
5. **Granularity** — slices are thin; no single slice bundles multiple independent deliverables that could ship separately. FAIL if any slice clearly should be split into two-plus tracer bullets. — quality
6. **Template + hygiene** — each issue carries the required sections (What to build, Acceptance criteria, Blocked by), is tagged HITL or AFK, uses the project's domain vocabulary, and avoids stale-prone specifics (file paths, code snippets) except a sanctioned prototype snippet. FAIL on a missing required section or leaked file paths/code. — quality

## External data

Self-contained for the core check: the source plan/PRD and the drafted slice list are both in the conversation context when to-issues invokes this gate (file read only if a reference is passed instead).

One soft prerequisite, for criterion 6's vocabulary sub-check only: the project's **domain glossary and ADRs**. Read them from the repo (file read, or `mcp__semble__search` if available) when a path is given or they are discoverable. **Not yet wired up** — if no glossary/ADRs are found, skip the vocabulary sub-check and note it; do not FAIL criterion 6 on vocabulary alone.

No issue-tracker access is needed in this mode (the check runs before publishing).

## Process

1. Read the source plan/PRD and the drafted slice list from context (or fetch the passed reference). Optionally load the domain glossary + ADRs.
2. Extract every requirement / user story from the plan into a checklist. Extract every slice as `{title, type, blocked_by, what_to_build, acceptance_criteria}`.
3. Run each criterion check, recording Pass/Fail and the specific offending slice(s):
   - Coverage: map requirements ↔ slices; record unmapped requirements and orphan slices.
   - Vertical: classify each slice as end-to-end vs single-layer.
   - Graph: hand the slices to `check_dag.py` and use its result — don't eyeball the edges. Emit each slice as `{ "id", "blocked_by": [...] }` (use the slice title or number as the id) and pipe the JSON list in:

     ```bash
     uv run .claude/skills/verify-issue-breakdown/scripts/check_dag.py -   # reads the JSON list from stdin
     ```

     It prints `{"acyclic": bool, "cycles": [[...]], "dangling": [[referrer, missing]...]}` and exits 0 for a clean DAG, 2 when there's any cycle or dangling ref. Criterion 3 is **Pass** iff `acyclic` is true and `dangling` is empty; otherwise **Fail**, and the offenders are the reported cycles and dangling pairs.
   - Verifiable / Granularity / Template: inspect each slice against its criterion.
4. Combine: PASS iff all six pass; otherwise FAIL, listing each failing criterion with the offending slice(s).
5. Render the verdict from `assets/report_template.md` — read it and fill the placeholders (see *Output format*).

## Output format

Read `assets/report_template.md` and fill its placeholders — the one-line verdict, the six-row criterion table, and the conditional offenders list. Don't reconstruct the scaffold from memory; the template is the single source of truth for the layout.

Fill the tokens:

- `{{VERDICT}}` — `PASS` or `FAIL`.
- `{{ONE_LINE_REASON}}` — the headline (e.g. "all 12 slices vertical, graph acyclic, plan fully covered"; on FAIL, the worst failing criterion).
- `{{PLAN_COVERAGE}}` … `{{TEMPLATE_HYGIENE}}` — `Pass` or `Fail` per criterion.
- `{{OFFENDERS}}` — empty on a clean PASS; on FAIL, the offenders only, one line each, slice → what's wrong → the fix:

  - **Slice 4 "Settings page"** — horizontal (UI only, no persistence path) → fold the save/read path in, or mark blocked-by the schema slice.
  - **Requirement "export to CSV"** — no slice covers it → add a slice.

Don't pad the report with criteria that passed cleanly.
