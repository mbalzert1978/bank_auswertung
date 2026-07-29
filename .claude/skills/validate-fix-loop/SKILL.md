---
name: validate-fix-loop
description: Dispatch the 23 `verifier-<smell>` code-smell checks listed in config.json's `validators` array in parallel, each as its own subagent — then, once every one has returned, dispatch each verifier's findings to its own paired `fixer-<smell>` skill, also as a subagent, in waves whose file sets are disjoint so no two fixers race on the same file. The orchestrator gathers every fixer's input for it (exact locations plus the current source excerpts) and runs the test gate itself once per iteration, so a fixer only edits. Repeat until every verifier comes back with zero findings, total findings stop dropping between iterations (plateau), or config.json's max_iterations is hit. Use when the user wants a "review-fix loop", "validate and fix until clean", "Qualitaets-Loop ueber den aktuellen Diff", "run all verifiers and fix findings until clean", or "lauf die Validatoren bis alles clean ist".
arguments: Optional. Scope to validate — a diff/branch/PR (default: current branch vs merge-base with the default branch).
---

# Validate-Fix Loop

Dispatcher + looper over the code-smell verifiers named in `config.json` and the
fixers that remediate their findings. This skill's one job is **coordination** — it
does not review, lint, test, or fix anything itself; every substantive check and every
edit is delegated to a subagent. See `.claude/skills/CLAUDE.md` for the bucket contract
this fits: **Orchestration** — coordinating the configured verifiers and fixers into a
repeatable loop is the single job, not a bucket straddle.

Nothing in this file — or in any verifier or fixer it dispatches — knows the repo's
language, stack, or conventions. The only repo-specific value is the test command, and
that lives in the `run-tests` skill's own `config.json`, reached through `test_gate`.

```
 ┌─► every verifier in config.json's `validators` (23 subagents, all parallel) ─┐
 │                                                                              │
 └──────────────────────────────────► all returned ─────────────────────────────┘
                                            │
                                            ▼
                     plan_iteration.py: parse · sum · stop? · route · pack into waves
                                            │
                                            ▼
        ┌───────────────────────────────────────────────────────────────────┐
        │ wave 1: fixers with pairwise disjoint file sets → parallel        │
        │ wave 2: the fixers that collided with wave 1 → parallel           │
        │ …                                              (waves in order)   │
        └───────────────────────────────────────────────────────────────────┘
                                            │
                                            ▼
                          test gate — ONCE, in the orchestrator
                                            │
 └──────────────── next iteration (unless clean, plateau or capped) ◄───────────┘
```

## Iron guards — check every one

- [ ] **Never dispatch any fixer before every configured verifier of the current
      iteration has returned.** Validate fully, then mutate — the same "no mutation
      before complete validation" guarantee a well-built batch-mutation system follows.
- [ ] **Every verifier and every fixer runs as its own subagent** via the Agent tool,
      with `model` set to `config.json`'s `agent_model`. Never invoke a verifier or a
      fixer through the Skill tool in this conversation — that would run it on the
      orchestrator's model and in the orchestrator's context, which is exactly what a
      31-way fan-out must not do. Never hard-code the model here; changing it later is a
      `config.json` edit.
- [ ] **Route by `config.json`'s `fixer_map`.** Each verifier's findings go to its own
      paired fixer and only its own — never bundle one verifier's findings into another
      fixer's dispatch. Adding a new smell pair later is a `validators` + `fixer_map`
      entry, not a change to this file.
- [ ] **Fixers run in the waves `plan_iteration.py` returns: parallel *within* a wave,
      sequential *across* waves.** The script guarantees a wave's file sets are pairwise
      disjoint, which is the whole reason parallel is safe there; two fixers editing the
      same file concurrently would race and silently lose one another's edits. Never
      merge waves, never reorder them.
- [ ] **The orchestrator hands each fixer everything it needs and gathers it fresh,
      immediately before that fixer's wave is dispatched** — never all upfront. An
      earlier wave has already edited the tree, so an excerpt gathered before it ran is
      stale, and a fixer working from a stale excerpt writes a broken edit.
- [ ] **The fixer does not search, does not re-run its verifier, does not run tests.**
      Its brief carries the located findings and the current source excerpts; it reads
      only the file it is about to edit (the harness requires a read before an edit) and
      edits it. Verification is the orchestrator's job, once per iteration.
- [ ] **The test gate runs exactly once per iteration, in the orchestrator, after the
      last wave** — not inside a fixer, not per wave. It is the only objective signal in
      this loop; a red gate ends the run and names the wave that broke it.
- [ ] **A verifier reporting `Verdict: CONFIG ERROR` ends the loop immediately** — no
      fixer dispatch, no further iterations. Only a human can supply missing config.
      Report which verifier(s) hit it and their message, verbatim, and stop.
- [ ] **The loop stops the instant any of these is met:** every verifier reports
      `Findings: 0` in the same iteration (checked first — a clean pass ends the loop
      even on iteration 1); the iteration's total findings did not decrease from the
      prior iteration's total (plateau); or `max_iterations` is reached.
- [ ] **A plateau ends the loop, same as a cap.** A verifier whose bar is judgement-heavy
      can keep finding *something* every iteration even as fixers make real progress
      elsewhere; looping to `max_iterations` against it is a goose chase. Comparing each
      iteration's **total** against the previous one catches this generically, without
      special-casing any one skill.
- [ ] **`max_iterations` and `agent_model` live in `config.json`.** Never hard-code
      either, never ask for them at invocation time.
- [ ] **Every configured verifier and fixer skill runs unmodified**, invoked by name —
      each is an existing skill wired in as-is, never reimplemented here.
- [ ] **Same scope for every verifier, every iteration**, re-resolved fresh each pass (a
      fixer wave may have changed the diff).

## Process

### 1. Resolve scope

Take scope (diff/branch/PR) from `arguments`. Default: the current branch's changes vs
the merge-base with the default branch.

### 2. Loop

Read `max_iterations`, `agent_model`, `test_gate` and the `validators` list from
`config.json`. Set `iteration = 1` and `previous_total = null`.

**a. Dispatch every configured verifier in parallel** — one Agent-tool call per
verifier, all in the same message so they run concurrently, foreground
(`run_in_background: false`; the loop cannot proceed until every one has returned),
`subagent_type: "general-purpose"` (it needs the Skill tool), `model:` `agent_model`.
For each, fill `assets/validator-brief.md`:

| Token | Value |
|-------|-------|
| `{{VALIDATOR_SKILL}}` | the verifier's name from `config.json` |
| `{{SCOPE}}` | the resolved scope |

**b. Collect.** Keep each returned report's full text verbatim, tagged by which verifier
produced it. Nothing to extract by hand — the next step's script does the
`Verdict`/`Findings` parsing, the arithmetic and the file-set grouping.

**c. Plan the iteration.** Run:
```bash
uv run .claude/skills/validate-fix-loop/scripts/plan_iteration.py
```
piping in JSON on stdin: `{"config_path": ".claude/skills/validate-fix-loop/config.json",
"iteration": <n>, "previous_total": <null or last iteration's current_total>, "reports":
[{"validator": "...", "report": "<that verifier's full report text, verbatim>"},
...one per configured verifier]}` (see the script's own docstring for the exact shape).
Its output is `{"stop": ..., "current_total": ..., "config_error_validators": [...],
"waves": [...]}` — use it as-is; don't re-derive the parsing, the sum, the
plateau/cap comparison, the fixer routing or the wave packing by hand.

- `stop == "config_error"` → loop ends immediately, **without** dispatching anything.
  Report **config error** using `config_error_validators`, stop.
- `stop == "clean"` → loop ends. Report **clean**, stop.
- `stop == "plateau"` → the last round of fixers produced no net improvement (or things
  got worse). Loop ends **without** dispatching any fixer again. Report **plateau** with
  this iteration's leftover findings, stop.
- `stop == "cap"` → loop ends. Report **cap reached** with the leftover findings, stop.
  `max_iterations` counts validation passes, not fixer rounds: the final permitted
  iteration always validates (so the report reflects the true current state) but never
  triggers another round of fixers — there'd be no next iteration left to confirm it
  helped. A `max_iterations` of `1` therefore runs one diagnostic-only pass with no fix
  attempt at all; the default of `3` allows up to two fixer rounds.
- `stop == null` → set `previous_total = current_total` and continue to step d with
  `waves`.

**d. Dispatch fixers, one wave at a time**, in the order the script returned them.
For each wave:

1. **Gather the wave's input, now** — not earlier. For every unit in the wave, read the
   files in its `files` list and cut the excerpts its findings point at (the located
   region plus enough surrounding context to apply the named refactoring). Doing this
   per wave rather than upfront is what keeps a later fixer from working against a tree
   an earlier wave already changed.
2. **Dispatch every unit of the wave in parallel** — one Agent-tool call per unit, all
   in the same message, foreground, `subagent_type: "general-purpose"`, `model:`
   `agent_model`. For each, fill `assets/fixer-brief.md`:

   | Token | Value |
   |-------|-------|
   | `{{FIXER_SKILL}}` | `unit.skill` |
   | `{{VERIFIER_SKILL}}` | `unit.validator` |
   | `{{REPORT}}` | `unit.validator`'s own report, verbatim — never a paraphrase, never mixed with another verifier's findings |
   | `{{FILES}}` | `unit.files`, one path per line |
   | `{{EXCERPTS}}` | the excerpts gathered in step 1, each under a `--- <path> (lines a–b)` header |

3. **Wait for every unit of the wave** to return its outcome table before gathering the
   next wave's input.

**e. Test gate.** After the last wave, if `test_gate.enabled`, run `test_gate`'s
`command` + `args` from the repo root, unmodified. Green → continue. Red → **the loop
ends here**: report which wave last touched the tree, paste the failure output verbatim,
and say plainly that the working tree is left mutated and needs a human. Do not attempt
to repair it by dispatching more fixers — a fixer only applies findings it was handed,
and nothing handed it this failure.

**f.** `iteration += 1`; go to **2a**.

### 3. Report

## Report format

One row per iteration, one column per verifier in `config.json`'s `validators` order
that reported a nonzero `Findings` count in at least one iteration of this run — with 23
configured verifiers, omit any column that stayed at `0` for the whole run rather than
padding the table; name which verifiers were clean throughout in one line instead.

| Iter | verifier-long-method | verifier-comments | verifier-dead-code | Fixer waves | Test gate |
| ---- | --------------------- | ----------------- | ------------------ | ----------- | --------- |
| 1 | 1 | 1 | 1 | W1: fixer-long-method (1 fixed) ‖ fixer-dead-code (1 fixed) · W2: fixer-comments (1 fixed) | green |
| 2 | 0 | 0 | 0 | — (clean) | — |

`Fixer waves` lists the waves in dispatch order, `‖` between units that ran in parallel
within a wave and `·` between waves, each as `<fixer>: <n> fixed` (plus
`skipped`/`no_change_needed` counts if any survived). `Test gate` is `green`, `red` or
`disabled via config`.

End with one of:
- **Clean after N iterations** — total findings fixed across the run.
- **Plateau after iteration N** — the previous round of fixers made no net progress; no
  fixer was run again on iteration N's findings. List the still-open findings, verbatim
  enough to hand to a human, and which verifier(s) they came from. Say plainly if one
  verifier looks like the one that won't converge — that's a signal to review by hand,
  not a loop bug.
- **Cap reached after `max_iterations` iterations** — the still-open findings and which
  verifier(s) they came from.
- **Test gate red after iteration N** — which wave last mutated the tree, the verbatim
  failure output, and that the tree is left dirty.
- **Config error in iteration N** — which verifier(s) returned `Verdict: CONFIG ERROR`
  and their message, verbatim; no fixer was dispatched. Not a code problem — say which
  `config.json` needs a human's attention.

Don't pad the report with iterations that didn't happen (a run that goes clean on
iteration 1 has one row, not `max_iterations` rows).
