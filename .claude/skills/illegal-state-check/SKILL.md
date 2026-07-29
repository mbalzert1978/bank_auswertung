---
name: illegal-state-check
description: Review a diff/branch for data models that let invalid states exist — a flag/enum plus optional fields only meaningful in certain combinations, an unconstrained primitive standing in for a constrained domain, a mutable object whose invariants can be violated between construction steps — and push to make the bad state unrepresentable instead of merely checked. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking a data/type model for representable-but-invalid states — "kann dieser Zustand ueberhaupt ungueltig sein", "make invalid states unrepresentable", "parse dont validate check", "illegal state check".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Illegal State Check

Judges the diff's data/type model by one question: can it represent a combination of
values that should never exist? Hold the model to the same bar as control flow — a
good model makes illegal states *impossible to construct*, not merely unlikely or
caught by a runtime check. This is deliberately narrower than a general design review
(`thermo-nuclear-code-quality-review`, standalone) or the SOLID/pattern/idiom splits:
the finding here is specifically about the *shape of the data*, not about behavior,
responsibility, or style.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)"). Identify the language(s) from the files under
review — this principle translates across languages (a sum type is a sealed class
hierarchy in Java/Kotlin, a discriminated union in TypeScript, an `enum` with
associated data in Rust/Swift, or a closed record hierarchy in C#).

## What to look for

- **Flag/enum plus a bag of optional fields only meaningful in certain combinations** —
  a `status` field next to a `result` that's only set when `status == done`, or an
  `error` only set when it failed. Every consumer has to re-derive which fields are
  live for the current status. The fix: model it as a closed set of cases (a sum
  type/discriminated union) where each case carries exactly — and only — the data
  valid for it.
- **Primitives standing in for a constrained domain** — a raw string/int where only a
  small set of values is legal, an unvalidated email/URL/id string, a pair of numbers
  that must satisfy an invariant like `start <= end`. The fix: *parse, don't
  validate* — a parsed/constrained type built once at the boundary, so an invalid value
  can never flow downstream; everywhere else in the code carries the already-valid
  type instead of re-checking it.
- **Mutable objects whose invariants can be violated between steps** — a half-built
  object that's usable before it's actually complete, or two fields that must change
  together but can be set independently (so a caller could set one and forget the
  other). The fix: enforce the invariant in the constructor/factory (build it complete
  or not at all), or make the object immutable so a "half-valid" instance can't exist
  at all.
- **Nullability used to paper over a missing case** — a nullable field that is really
  "not applicable in some states" rather than genuinely optional; every consumer now
  needs a null check that's really a disguised state check. The fix is usually the
  same sum-type fix as the first bullet.

Don't accept "but we check for it" as a defense when a bad state is representable —
that's exactly the case to push on: if a tighter model that forbids the invalid
combination is available, the runtime check should disappear entirely, not just get
centralized. Treat this as being just as demanding as a spaghetti-code finding: a
closed set of states encoded as loosely-coupled optional fields (or a bare primitive),
when a sum type / parsed type is clearly available, is a presumptive finding — not a
stylistic nit.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Kind | Illegal state representable | Location | Fix |
| ---- | ---------------------------- | -------- | --- |
| Flag + optional fields | `Status` enum next to nullable `Result`/`Error`, only one ever valid at a time | `datei:zeile` | closed sum type per status, each carrying only its own data |
| Unconstrained primitive | raw `string` used as an email with no parsing at the boundary | `datei:zeile` | parsed `EmailAddress` value type, constructed once |
| Violable invariant | `start`/`end` fields settable independently, no order enforced | `datei:zeile` | validate in the constructor, or a single `Range.Create(start, end)` factory |

Only list rows for genuine, concrete illegal states — don't pad with hypothetical
"could theoretically be tighter" nits on a model that's already sound. End with:

```
Findings: <n>
```

`<n>` = count of concrete illegal-state findings.
