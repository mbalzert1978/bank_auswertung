---
name: multi-agent-thermo-nuclear-review
description: Manual-only orchestration wrapper that runs the thermo-nuclear-code-quality-review as a multi-agent fan-out — per configured lens, several diverse finder agents plus one adversarial verifier run in parallel in the background, then the agent reconciles the result against its own reading before answering. Invoke ONLY by explicitly typing /multi-agent-thermo-nuclear-review; this skill has no automatic natural-language trigger and must never be auto-dispatched.
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes when omitted.
---

# Multi-Agent Thermo-Nuclear Review

A manual-only orchestration wrapper around `thermo-nuclear-code-quality-review`. It does **not**
re-implement the review: it fans the same strict, structure-first audit out across several
configured lenses, each reviewed in parallel by diverse finder agents and an adversarial verifier,
and ends with **one highest-impact finding per lens** that the orchestrating agent has reconciled
against its own reading of the code.

It **delegates** to `thermo-nuclear-code-quality-review` (read-only, by path) and never edits it:
- the review **standard** — the finders/verifiers read its `SKILL.md` rubric;
- the objective **size pass** — its `scripts/changed_files.py`.

Lenses, their angles, the finder count, and the project guardrails are all configured in
`config.json`; the prompt scaffolding and the structured-output schemas live in `assets/`.

## Process

1. **Resolve the scope.** From the invocation `arguments`: a file/dir path, a PR number, or a base
   branch / diff range. If nothing was passed, ask with `AskUserQuestion` (or default to the current
   branch's changes vs. its merge-base with the default branch — but say which you chose).

2. **Build the workflow args (deterministic).** Run from the repo root:
   ```bash
   uv run .claude/skills/multi-agent-thermo-nuclear-review/scripts/prepare_args.py --scope "<scope>"
   ```
   It reads `config.json` (lenses, `finder_count`, guardrails) + `assets/` (both schemas + the prompt
   templates), validates them, and emits the finished `args` JSON to stdout. If it exits non-zero,
   surface the error and stop — do not start the workflow with a broken config.

3. **Run the delegated size pass.** Run `thermo-nuclear-code-quality-review`'s objective check by path:
   ```bash
   uv run .claude/skills/thermo-nuclear-code-quality-review/scripts/changed_files.py [scope]
   ```
   Any `OVER`/`CROSSED` flag feeds the reconciliation in step 7.

4. **Read the scope yourself.** Read the files / diff under review in full. This is the basis for
   reconciling the agents' findings against your own reading — it is the core value, not optional.

5. **Make the run visible.** Emit this line verbatim (adjust the counts only if `config.json` deviates
   from 3 lenses × 3 finders — it renders `lenses × finder_count` finders and `lenses` verifiers):

   > Der Multi-Agent-Review läuft im Hintergrund (9 Finder + 3 adversariale Verifier, je einer pro Betrachtungswinkel). Ich warte auf das Ergebnis und gleiche es dann gegen meine eigene Lesart ab, bevor ich antworte.

   Then add a short note that the live progress is followable under `/workflows`.

6. **Start the workflow in the background.** Pass the step-2 JSON straight through as `args`:
   ```
   Workflow({
     scriptPath: '.claude/skills/multi-agent-thermo-nuclear-review/scripts/multi_agent_review.js',
     args: <prepare_args.py output>,
     run_in_background: true,
   })
   ```
   Wait for the `<task-notification>` — do **not** poll. The run reports its `Find` / `Verify` phases
   per lens live in `/workflows`, so the user sees it is still working.

7. **Reconcile against your own reading.** For each per-lens finding from the workflow, check it against
   step 4: override the framing or priority where justified, and drop a finding that collides with a
   guardrail or does not hold when you re-read the cited `file:line`. Fold in the size-pass result from
   step 3.

8. **Report** (format below).

## Report format

Exactly one finding per lens, as a table:

| Lupe | Befund (file:line) | Verbesserung | Priorität |
|------|--------------------|--------------|-----------|
| … | … | … | hoch/mittel/niedrig |

Then two short lines:
- a **calibration line** — how strict/confident the run was, and what the verifiers rejected;
- a **„Bewusst nicht geflaggt"** line — deliberate decisions (guardrail hits) intentionally not
  counted as findings.

No `Verdict:` line — this wrapper delivers per-lens findings, not the base skill's `BLOCK`/`APPROVE` gate.
Don't pad the report: skip a lens's row only if its verifier returned the explicit nothing-real state
(`outcome.found:false`), and say so.
