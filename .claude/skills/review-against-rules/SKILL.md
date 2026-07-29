---
name: review-against-rules
description: Review the current diff/branch against this repo's configured coding-standard directories and, if configured, its reference implementation, via the existing `senior-code-reviewer` subagent, and return its BLOCK/APPROVE verdict plus a machine-checkable `Findings: <n>` count. Portable across repos via `config.json` (`rules_dirs`, optional `reference_implementation`) — required config missing ends the run with `Verdict: CONFIG ERROR`, not a silent pass. Use as one leg of a validator loop, or standalone when checking a change against this repo's coding standards and reference implementation — "prüft der Code gegen die Regeln und die Referenzimplementierung", "review against the configured rules and reference".
arguments: Optional. Scope — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes vs its merge-base with the default branch when omitted.
---

# Review Against Rules

Delegates the actual review to the existing `senior-code-reviewer` subagent — this skill
does not reimplement a review rubric. Its one job is to **point** that agent at what a
generic review would miss: this repo's own coding-standard directories, and — if this
repo has one — a reference implementation, then relay the verdict in a form a loop can
check mechanically.

Everything repo-specific lives in `config.json`, not in this file, so the skill works
unmodified in another repo — only its `config.json` changes.

## Process

0. **Preflight.** Read `config.json`. `rules_dirs` is required — a review against zero
   configured rule directories isn't this skill's job, it's just a generic review, and
   silently falling back to a hardcoded default (e.g. `.rules/`) would be a false
   positive in a repo that doesn't use that convention. If `rules_dirs` is missing or
   empty, **stop here** without dispatching any subagent and report:

   ```
   Verdict: CONFIG ERROR
   `rules_dirs` ist in .claude/skills/review-against-rules/config.json nicht gesetzt (oder
   leer) — review-against-rules kann ohne konfigurierte Regel-Verzeichnisse nicht sinnvoll
   pruefen. Bitte `rules_dirs` konfigurieren (Liste von Pfaden zu den Coding-Standards
   dieses Repos).
   ```

   `reference_implementation` stays optional — see step 2.

1. **Resolve scope** — follows `_shared/validator-contract.md` ("Scope resolution
   (diff-scoped validators)").

2. **Build the brief.** Read `assets/agent-brief.md` and substitute:

   | Token | Value |
   |-------|-------|
   | `{{SCOPE}}` | the resolved scope |
   | `{{RULES_DIRS}}` | `rules_dirs` from `config.json`, one path per line |
   | `{{REFERENCE_SECTION}}` | see below |

   `{{REFERENCE_SECTION}}` is either the filled block below (if `config.json` has a
   `reference_implementation` with a non-empty `paths`), or an **empty string** if not —
   omit it entirely, don't leave a dangling empty heading:

   ```
   - **The reference implementation** — {{REFERENCE_NOTE}}
     Reference files:
   {{REFERENCE_PATHS}}
     Where the change under review reimplements logic that exists in these files, verify
     it was carried over faithfully (same safety nets, same invariants) rather than
     reinvented or weakened.
   ```

   where `{{REFERENCE_PATHS}}` is `reference_implementation.paths` (one path per line) and
   `{{REFERENCE_NOTE}}` is `reference_implementation.note`.

3. **Dispatch.** Run the filled brief through the Agent tool with `subagent_type:
   "senior-code-reviewer"`, in the foreground (this skill needs the result before it can
   report). Do not paraphrase the rubric yourself — the agent carries it.

4. **Relay + append the count.** Print the subagent's output verbatim (the `Verdict:`
   line plus its findings), then append one more line:

   ```
   Findings: <n>
   ```

   `<n>` is the number of findings actually listed under the verdict — `0` when
   `Verdict: APPROVE` listed none. Count, don't guess: if the agent's `APPROVE` still
   lists nice-to-have items, `<n>` is that count, not `0` — a caller looking only at
   `Findings:` must never miss a real item.

## Report format

Whatever `senior-code-reviewer` returned, verbatim, plus the trailing `Findings: <n>`
line described above. Nothing else — this skill does not add its own summary on top of
the subagent's judgment. For a step-0 abort, the `Verdict: CONFIG ERROR` block instead,
per `_shared/validator-contract.md` ("`Verdict: CONFIG ERROR` abort").
