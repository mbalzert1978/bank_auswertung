# Fixer Contract (shared)

Every `fixer-<smell>` skill applies the fix for the one code smell its paired
`verifier-<smell>` skill detects. Referenced by name from each fixer's own
`SKILL.md` instead of restated — the same one-source-of-truth convention as
`_shared/validator-contract.md`. Edit this file, not a per-skill restatement
of it.

A fixer never re-invents what counts as an instance of its smell — that
judgment belongs to the paired `verifier-<smell>` skill. A fixer's own job is
narrower: take a located instance and apply the specific refactoring technique
already named for it.

Nothing in a fixer may assume a language, a stack, or a particular repo's
conventions. Every refactoring named here is structural and translates across
languages; where a fix depends on a repo-specific decision, that decision comes
from the finding, not from the fixer.

## Input resolution

Resolve what to fix from `arguments`, in this order:

1. **A `verifier-<smell>` report was handed over** (pasted text, or a path
   containing one) — parse its rows directly: location, defect, and the `Fix`
   the verifier already named. This is the normal case, and the handover is
   **authoritative**: never re-derive a different fix than the one the verifier
   stated, and never go looking for instances it didn't list.
2. **Only a target was given** (file/dir path, PR number, diff range) with no
   report — run the paired `verifier-<smell>` over that target first, before
   touching anything ("no mutation before complete validation"). If it comes
   back `Verdict: APPROVE` (zero findings), stop and say so — there is nothing
   to fix.
3. **Nothing was given** — resolve scope exactly as the paired
   `verifier-<smell>` skill does (current branch's changes against its
   merge-base with the default branch), then proceed as in 2.

## Process

1. Resolve the input findings per the rules above.
2. For each finding, apply **exactly** the refactoring technique already named
   for it — the specific mechanic in the fixer's own "Refactorings to apply"
   section, not a generic cleanup. Keep each fix scoped to that one finding;
   don't fold in unrelated cleanup while you're in the file.
3. Read a file in your own context immediately before editing it (the harness
   requires it, and it is also how you notice that an earlier edit already
   changed the region). If the located instance no longer matches what the file
   actually says, trust the file: adapt the fix, or mark the finding
   `no_change_needed` with that reason rather than forcing the edit.
4. If two findings conflict (same location, incompatible fixes — rare), say so
   in the report and pick the one from the more specific finding rather than
   silently applying one.

## What a fixer does not do

These belong to the caller, and doing them here duplicates work that has
already happened or is about to:

- **Don't run the test suite.** When dispatched by `validate-fix-loop`, the
  orchestrator runs the test gate once per iteration after every fixer has
  returned; a suite run per fixer multiplies that by the number of fixers.
  Invoked standalone, the human decides when to run tests.
- **Don't re-run the paired verifier to confirm your own fix.** The loop
  re-validates on its next iteration, which is the same check one round later
  and against the *whole* tree rather than your slice of it.
- **Don't search for further instances.** The findings you were handed are the
  scope. Anything else belongs in the next validation pass.
- **Don't stray outside the files the findings name.** If a named refactoring
  genuinely forces an edit elsewhere (a call site that must change for the code
  to remain valid), make it and **say so explicitly** — a caller may have
  scheduled other fixers in parallel on the assumption that your file set is
  yours alone.

## Report format

One row per finding — nothing dropped silently:

| Finding | Refactoring applied | Outcome | Note |
| ------- | -------------------- | ------- | ---- |

`Outcome` is one of: `fixed`, `skipped` (with a reason in `Note`), or
`no_change_needed` (a closer look showed it wasn't a genuine instance — say
why). Name any file you had to touch beyond the ones the findings pointed at.
End with the totals (`fixed` / `skipped` / `no_change_needed`), then:

```
Fixed: <n>
```

`<n>` = count of findings with outcome `fixed`.
