---
name: architecture-adr-check
description: Check code changes against this repo's architecture-decision docs and the issue they implement — both located via `config.json` (`adr_dir`, `issues_dir`), not a hardcoded path — does the diff honor every relevant decision's invariant, and does it satisfy the issue's acceptance criteria — and return PASS/FAIL plus a `Findings: <n>` count. Required config missing ends the run with `Verdict: CONFIG ERROR`, not a silent pass over an empty doc set. Use as one leg of a validator loop, or standalone when checking a change against its architecture decisions/issue — "haelt sich der Code an die ADRs", "erfuellt das die Akzeptanzkriterien vom Issue", "architecture check against the configured decision docs and the issue".
arguments: Optional. Scope (diff/branch/PR, default current branch vs merge-base with main) and the linked issue — an id (e.g. `0026`) or path. If the issue is omitted, the skill ranks open issues by keyword overlap with the changed paths and either picks an unambiguous top match or asks.
---

# Architecture / ADR / Issue Check

Two things a generic code review won't check: whether the diff honors every architecture
decision whose invariant the changed area actually touches, and whether it satisfies the
acceptance criteria of the issue it implements. Both require reading this repo's own
documents, not a general rubric — so unlike most of the other validators, most of the
work here is judgement; the bundled script only does the mechanical lookup, at the paths
`config.json` names — nothing here is hardcoded to `docs/adr/`/`docs/issues/`, those are
just this repo's chosen values for `adr_dir`/`issues_dir`.

## Process

0. **Preflight.** Read `config.json`. Both `adr_dir` and `issues_dir` are required — a
   silent fallback to a hardcoded default like `docs/adr` would look plausible but be
   wrong in a repo that organizes its decision docs differently, and would produce a
   false PASS (empty doc set, nothing to violate) instead of a visible error. If either
   key is missing or empty, **stop here** and report:

   ```
   Verdict: CONFIG ERROR
   `adr_dir` und/oder `issues_dir` sind in
   .claude/skills/architecture-adr-check/config.json nicht gesetzt —
   architecture-adr-check kann ohne beide Pfade nicht sinnvoll pruefen. Bitte beide
   Schluessel konfigurieren.
   ```

   (Missing either key aborts the whole skill, not just the affected half — both checks
   below depend on being able to trust that an empty result means "genuinely nothing
   relevant," not "misconfigured.")

1. **Resolve scope** — follows `_shared/validator-contract.md` ("Scope resolution
   (diff-scoped validators)").

2. **Resolve the issue.** If `arguments` names one, read it directly (`<issues_dir>/<id>-*.md`
   or the given path). Otherwise:
   ```powershell
   uv run .claude/skills/architecture-adr-check/scripts/find_relevant_docs.py issues \
     --status open --match <changed-path-1> <changed-path-2> ...
   ```
   Pass every changed file/dir path as a `--match` argument. If exactly one issue scores
   above zero and clearly above the rest, use it. If several tie or none score, ask via
   `AskUserQuestion` (header `Issue`) rather than guessing which one the diff implements.

3. **List the ADRs.**
   ```powershell
   uv run .claude/skills/architecture-adr-check/scripts/find_relevant_docs.py adrs
   ```
   Read every ADR whose title or content plausibly bears on the changed area — this repo
   has ~10 ADRs, so read all of them rather than pre-filtering by keyword; the script's
   `issues --match` ranking is a starting point for the issue lookup only, not a filter
   for which ADRs matter (an ADR like 0003's vertical-slice-in-one-module or 0009's
   layering can be violated by a change that has nothing to do with its title).

4. **Judge.** For each ADR whose decision applies to the changed area, and for each
   acceptance criterion in the resolved issue, decide: satisfied, violated (cite the
   `datei:zeile` and the exact clause it breaks), or not-applicable to this diff. An ADR
   marked as revised/superseded (see its own header, e.g. ADR-0010's "Revision" note)
   is judged by its **current**, not historical, decision.

## Report format

**Verdict: PASS** or **Verdict: FAIL** (FAIL on any violated ADR invariant or unmet
acceptance criterion), then:

| Source | Item | Status | Location | Note |
| ------ | ---- | ------ | -------- | ---- |
| ADR-0010 | all-or-nothing rollback | satisfied | — | — |
| Issue #0026 | `.txt`-Strategie: Trenner konfigurierbar, Default `;` | violated | `Batch/TxtBatchTauschPaarQuelle.cs:12` | Default ist `,` statt `;` |

Only list `violated` and genuinely uncertain rows in full detail; `satisfied` rows may be
one line. Don't pad with every ADR that plainly doesn't apply — say which ones you judged
not-applicable in one line, not a row each. End with:

```
Findings: <n>
```

`<n>` = count of `violated` rows (unmet acceptance criteria + broken ADR invariants).

For a step-0 abort, the `Verdict: CONFIG ERROR` block instead, per
`_shared/validator-contract.md` ("`Verdict: CONFIG ERROR` abort").
