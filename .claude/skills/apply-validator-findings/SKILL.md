---
name: apply-validator-findings
description: Take the findings already produced by a set of quality validators (review-against-rules, thermo-nuclear-code-quality-review, lint-and-format-check, qa-check, architecture-adr-check) and fix every one, applying each finding's own located remedy — never re-reviewing, never inventing scope beyond what was flagged. Use when handed one or more validator reports (directly, or via the `validate-fix-loop` orchestrator) and asked to address every finding.
arguments: The validator report(s) to act on — pasted text, or a path where they were written — optionally with a stated priority order for conflict resolution (see step 2). Must contain at least one report with at least one finding.
---

# Apply Validator Findings

Consume already-produced findings and remediate them. This skill does not review,
does not decide what's in scope, and does not go looking for problems the validators
didn't flag — that separation (validate fully, *then* mutate) is deliberate, the same
"no mutation before complete validation" guarantee this repo's own batch-tausch scripts
follow. If you find yourself wanting to fix something no report mentioned, leave it and
say so; that belongs in the *next* validation pass, not this fix pass.

## Process

1. **Parse every finding** out of the handed-over report(s): its source validator, its
   location (`datei:zeile` or equivalent), the defect, and the remedy the validator
   already named. A finding with no concrete location or remedy can't be safely
   auto-applied — mark it `skipped` with the reason (`no located fix`) rather than
   guessing at one.

2. **Resolve conflicts before editing.** If two findings target the same location with
   incompatible fixes (rare — validators check different things), break the tie by
   priority order. When dispatched by `validate-fix-loop`, that order is handed to you
   directly alongside the reports (its `validators` list, in priority order — earlier
   entries, rules/architecture correctness concerns, outrank later ones, style); use it.
   When invoked standalone with no priority order given, fall back to the order the
   reports were handed to you in, and say in the report that the ordering was assumed
   rather than given — don't reach for a repo-specific priority you were never told.
   Don't silently apply one fix without noting the conflict either way.

3. **Apply each fix**, using the remedy the finding already specifies. Keep each fix
   scoped to that finding — don't fold in unrelated cleanup while you're in the file.

4. **Verify nothing broke.** After applying the batch, run:
   ```powershell
   uv run .claude/skills/run-tests/scripts/run-tests.py
   ```
   If a fix (or the batch) breaks the build or a test, don't leave it broken: retry that
   one fix, or revert just that change and mark its finding `skipped` with the reason.
   Never end this skill with a red build/suite.

## Report format

One row per finding — nothing dropped silently:

| Validator | Finding | Outcome | Note |
| --------- | ------- | ------- | ---- |
| review-against-rules | `Foo.cs:42` missing OldMac verification | fixed | — |
| lint-and-format-check | `Bar.cs` needs formatting | fixed | — |
| architecture-adr-check | ADR-0010 rollback order violated | skipped | fix would require a design decision — see note |

`Outcome` is one of: `fixed`, `skipped` (with a reason in `Note`), or `no_change_needed`
(the finding didn't survive a closer look — say why). End with the totals (`fixed` /
`skipped` / `no_change_needed`) and confirm build+tests are green, or name what's still red.
