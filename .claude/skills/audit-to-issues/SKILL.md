---
name: audit-to-issues
description: Audit a repository on a chosen quality axis and turn each independent finding into a clean, reviewed, deduped GitHub issue — one finding, one issue. Coordinates the audit (delegating over-engineering to the `ponytail-audit` plugin, owning the "make illegal states unrepresentable" rubric), per-finding parallel drafting, the `verify-issue-breakdown` quality gate, dedupe against open issues, and publish-last. Use when the user wants to "audit the repo and turn the findings into issues", "prüfe das Repository auf Over-Engineering / nicht-repräsentierbare Invalid States", runs a "pythonic-refactor-audit → issues", or asks for a "make illegal states unrepresentable" audit that ends in filed issues.
arguments: Optional. The audit axis/axes (`over-engineering` | `illegal-states` | `pythonic`, one or more) and the scope/path to audit. If the axis is omitted, ask via AskUserQuestion; scope defaults to the whole repository.
---

# Audit to Issues

Audit a repository on a chosen quality axis, then turn each independent finding into a
clean, reviewed, deduped issue on the tracker. This skill's **one job is the coordination**:
it runs the audit once, fans the findings out into parallel per-finding drafting, gates the
drafts, and publishes last. It does **not** re-implement the audit engine or the issue
machinery — it owns the *findings* (especially the illegal-states rubric) and the
*one-finding→one-issue* dispatch + dedupe, and delegates everything else.

```
audit (once, sequential) ─► per-finding parallel draft ─► gate (review+fix) ─► dedupe ─► publish (last)
        │                                                        │
   ponytail-audit / inline rubric                        verify-issue-breakdown (via to-issues)
```

Front of the pipeline: **audit-to-issues → `to-issues` (exists) → `issues-to-prs`**. This skill
audits *code* and files issues; it does not break an existing plan into issues (that is
`to-issues`) and it does not implement issues (that is `issues-to-prs`).

## Iron guards (do not violate)

- [ ] **One concern per issue.** Each finding becomes exactly one issue. Never bundle two
      findings into one issue; never split one finding across two.
- [ ] **Order is draft → review → fix → publish.** Never post a raw audit dump as issues.
- [ ] **Dedupe before you create.** Run `gh issue list --state open` and drop any finding
      that already has an open issue — skip it and report it, do not file it again.
- [ ] **Publish is the last step**, and only after the cleaned drafts PASS the gate. Never
      `gh issue create` before the gate passes.
- [ ] **Preserve the repo's language.** If the repo/issues/user write in German, the issues
      are German; if English, English. Detect from existing issues / README / the request.

## Audit axes

Pick one or more (multi-select). The axis decides the rubric and who runs it.

### `over-engineering` — what to delete / simplify / replace with stdlib

**Delegate to the `ponytail-audit` plugin skill** when installed — invoke it over the scope and
take its ranked findings (`<tag> <what to cut>. <replacement>. [path]`, tags `delete:` `stdlib:`
`native:` `yagni:` `shrink:`). One ponytail line = one finding = one issue.

If `ponytail-audit` is **not** installed, run the same audit inline: hunt deps the stdlib/platform
already ship, single-implementation interfaces, factories with one product, wrappers that only
delegate, dead flags/config, hand-rolled stdlib — rank biggest cut first. Do not hard-fail on the
missing plugin.

### `illegal-states` — make illegal states unrepresentable (this skill owns this rubric)

**The unique contribution.** A finding is a *single place where an invalid state is currently
constructible* and should instead be excluded by the type system / design — i.e. code where the
compiler/runtime would happily build a value that the domain says cannot exist. "Validated at
runtime everywhere" is not the same as "unrepresentable"; this axis hunts the latter.

Hunt for these constructible-illegal-state smells; each occurrence is one finding, paired with the
type/design change that makes the bad state impossible:

- **Primitive obsession over a constrained domain** — raw `str`/`int` where only some values are
  valid (email, non-negative age, ISO code, percentage). → a validated value object / newtype /
  branded type built through a smart constructor.
- **Enum + bag of nullable side-fields, only valid in some combinations** — fields that are only
  meaningful for one variant. → a sum type / sealed (discriminated-union) hierarchy where each case
  carries exactly its own data.
- **Boolean blindness** — several `bool`s (or a `bool` + nullable) encoding a state machine where
  only some combinations are legal. → one enum / ADT with only the legal states.
- **Optional-until-a-phase fields** — a field nullable at construction but required once the object
  is "ready". → split into distinct types (e.g. `Draft` vs `Published`); *parse, don't validate*.
- **Constructors / setters that allow an invalid intermediate** — public ctor or mutable setters
  that let a half-built or contradictory object exist. → private ctor + factory returning a
  `Result`/`Option`, immutable records, builder that only yields valid objects.
- **Stringly-typed closed sets** — a `string` parameter compared against a fixed list of literals.
  → enum / union.
- **Scattered invariant checks** — the same `assert`/guard repeated at every use site instead of
  enforced once at the boundary. → validate once at the edge, then carry a type that can't be wrong.
- **Possibly-empty where non-empty is required** — a list that must have ≥1 element typed as a
  plain list. → `NonEmpty`/`NonEmptyList`.
- **Fields that must agree but can drift** — `currency` + `amount`, `start` + `end`, `unit` +
  `value` modeled as independent fields. → bundle into one type that enforces the relationship.

Each finding names: the file/location, *which illegal state is constructible there*, and the
*type/design change* that excludes it.

### `pythonic` — pythonic / declarative refactor

Idiom and declarativeness (distinct from `over-engineering`'s "delete it"): imperative
`for`+`append`/`dict[...]=` loops that want comprehensions or generators; manual grouping/counting
that wants `itertools`/`collections`; hand-rolled context handling that wants `with`; ad-hoc record
classes that want `dataclass`/`NamedTuple`/`enum`; path string-mangling that wants `pathlib`;
procedural state that reads better as a declarative pipeline. One refactor site = one finding, with
the idiomatic form named.

## Process

1. **Resolve axis + scope.** Read them from `arguments`. If the axis is missing, ask with
   **AskUserQuestion** (`header: "Audit axis"`, options `Over-engineering` /
   `Illegal states unrepresentable` / `Pythonic / declarative`, **multiSelect: true** — the axes
   are independent). Scope defaults to the whole repo.

2. **Audit once, sequentially.** For each chosen axis run its audit over the scope (delegating
   `over-engineering` to `ponytail-audit`; owning `illegal-states` and `pythonic` inline). Produce a
   single flat list of **independent findings**, each `{ id, axis, location, problem, fix }`. This
   is the only sequential step — everything downstream is per-finding and parallel.

3. **Dedupe against existing open issues.** `gh issue list --state open --limit 200` (and a
   `--search` on key terms when the list is long). Drop any finding that an open issue already
   covers; record it as `deduped → #N` in the report. Only the survivors proceed. *(This dedupe is
   this skill's addition — `to-issues` does not do it.)*

4. **Draft one issue per finding, in parallel.** Findings are independent → fan out **one agent per
   finding** (reuse the `dispatching-parallel-agents` plugin pattern when present; otherwise
   dispatch inline with the Agent/Task tool — do not hard-fail if the plugin is absent). Each agent
   drafts exactly **one** issue for its finding, in the repo's language, on the `to-issues` issue
   template (What to build / Acceptance criteria / Blocked by). One concern only — an agent that
   finds its finding is really two files two issues, not one mixed issue.

5. **Gate, then fix (review step).** Collect the drafts and run the `verify-issue-breakdown` skill
   over the set — the same PASS/FAIL gate `to-issues` uses. It objectively checks coverage (every
   finding mapped, no orphan issue), one-concern granularity, independently-verifiable acceptance
   criteria, and template/hygiene. **FAIL → fix the offending drafts and re-gate (cap 3 attempts),
   then surface anything unresolved. Never publish on a FAIL.** This *is* the "review the issue text
   like source, fix the findings" step — see the next section for why it is `verify-issue-breakdown`
   and not `thermo-nuclear-code-quality-review`.

   > `verify-issue-breakdown` is built to gate *feature* decompositions; here it is reused past its
   > design center on refactor findings — deliberately, to avoid duplicating it with a bespoke gate.
   > So read its criteria sensibly: a refactor finding is "vertical" when it is a complete,
   > independently-demoable change for that one finding (code + its test), even though it does not
   > span schema/API/UI. The load-bearing criteria here are coverage, granularity (one concern),
   > testable acceptance criteria, and template/hygiene.

6. **Publish last.** Only after PASS: one `gh issue create` per surviving finding, applying the
   project's triage label, in dependency order if any finding blocks another. Reuse `to-issues`'
   publish path — do not re-invent issue creation. Report what was filed, what was deduped, and what
   was left unresolved by the gate.

## Reusing `to-issues` — and why no thermo-nuclear text review

This skill **delegates issue drafting, the quality gate, and publishing to the `to-issues` machinery**
(template → `verify-issue-breakdown` gate → `gh issue create`). It only front-loads the audit, the
dedupe, and the per-finding parallel drafting.

The user's original hand-template reviewed the issue *text* with
`thermo-nuclear-code-quality-review` and then filed issues manually. **This skill drops that step**,
deliberately:

- `thermo-nuclear-code-quality-review` is a **C#/.NET source-code** maintainability review
  (abstraction quality, file size, spaghetti conditionals). Pointed at prose issue text it is
  off-label and produces noise.
- Its only intent that *is* relevant here — DRY, clear structure, **one concern per issue, no mixing
  of findings** — is already an explicit, objective PASS/FAIL criterion inside
  `verify-issue-breakdown` (its *Granularity* and *Template + hygiene* checks). Purpose-built for
  exactly this gate.
- Running both would be the silent double-review this design forbids.

**Resolution: one gate, `verify-issue-breakdown`** (step 5). No thermo-nuclear pass on issue text.

## Graceful degradation

The plugin fallbacks live where the delegation happens — `ponytail-audit` in the `over-engineering`
axis, `dispatching-parallel-agents` in step 4 — so there is one source of truth per fallback. The
invariant across both: **never hard-fail on a missing plugin** — degrade to the inline path at that
call site and note it.

One case has no inline home: **no issue tracker / `gh` unauthenticated → stop before publishing** and
hand back the gated drafts so the user can file them; never silently skip dedupe or the gate.

## Report format

End with a short table — one row per finding, skipping nothing material:

| # | Axis | Finding | Outcome |
|---|------|---------|---------|
| 1 | illegal-states | `Money(amount, currency)` lets the two drift | filed → #42 |
| 2 | over-engineering | `IFooFactory` has one product | deduped → #31 |
| 3 | pythonic | manual count loop in `report.py` | gate FAIL ×3 → needs user |

Close with the one highest-value next action (e.g. "3 issues filed; #2 already existed; finding 3
still bundles two refactors — split it before filing"). Don't pad with steps that passed cleanly.
