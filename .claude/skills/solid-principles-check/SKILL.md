---
name: solid-principles-check
description: Review a diff/branch against the five SOLID principles (Single Responsibility, Open/Closed, Liskov Substitution, Interface Segregation, Dependency Inversion), each judged against its own named smell checklist, plus a mechanical file-size pre-check (an oversized file/type is a classic SRP smell). Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking SOLID conformance — "haelt sich das an SOLID", "SOLID-Check", "verletzt das Single Responsibility", "does this violate dependency inversion".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# SOLID Principles Check

Judges a diff against the five SOLID principles by name — not a general "is this
clean" review (that's `thermo-nuclear-code-quality-review`, standalone; or
`design-pattern-fit-check`/`language-idiom-check`/`illegal-state-check` for the other
split-out lenses). Each of the five gets its own concrete checklist below; apply
whichever ones the changed code actually touches — a small diff might only implicate
one or two letters, and that's fine, don't force all five onto every finding.

Identify the language(s) from the files under review and apply each principle in that
language's idiom — these are structural principles, not syntax, so they translate
across languages (a "fat interface" is a Java/C# `interface`, a Python `Protocol`, a
Rust `trait`, or a TypeScript `interface`, whichever the codebase uses).

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

Run the bundled file-size pass **before** reviewing — the one fully objective check
here, so don't eyeball it:

```bash
uv run .claude/skills/solid-principles-check/scripts/file_size_check.py [target]
```

It lists every file under review with its old/new line counts and flags any that
exceed (`OVER`) or just crossed (`CROSSED`) the threshold in `config.json`'s
`file_size_warn_lines`. A flagged file is a **candidate** SRP violation, not an
automatic one — judge whether the size actually reflects mixed responsibilities (a
long file that is one cohesive concern, e.g. a big generated mapping table, isn't an
SRP violation) before counting it as a finding.

## The five checks

### S — Single Responsibility

One reason to change. Watch for:
- A god class/method/file that mixes unrelated concerns (persistence + validation +
  formatting in one place).
- A method that reads like a table of contents ("validate, then transform, then save,
  then notify") — each step is a different responsibility bundled into one call.
- The file-size flag above, when the size actually reflects mixed concerns rather than
  one long but cohesive routine.

### O — Open/Closed

Extend behavior without modifying stable, already-working code. Watch for:
- A new `if`/`else if`/`switch` arm bolted onto an existing conditional to handle a new
  case, instead of a new implementation of an existing seam (a new `Strategy`/handler/
  polymorphic case).
- Modifying a shared function's internals to special-case a new caller, instead of the
  function accepting an extension point (a parameter, a policy, an injected behavior).

### L — Liskov Substitution

A subtype/implementation must be usable wherever its supertype/interface is expected,
without surprising the caller. Watch for:
- An override that throws `NotImplementedException` (or the language's equivalent) for
  a method its interface promises works.
- An override that strengthens preconditions (rejects inputs the base type accepts) or
  weakens postconditions (returns something less complete than callers expect).
- Callers that type-test-then-cast to a concrete subtype to work around a base-type
  method that doesn't actually behave uniformly — a sign the substitution already
  broke.

### I — Interface Segregation

Consumers shouldn't depend on more surface than they use. Watch for:
- A fat interface where most implementers stub out several members with
  `NotImplementedException`/`throw`/no-ops.
- A new method added to a broad shared interface for the sake of one caller, forcing
  every other implementer to grow a stub.

### D — Dependency Inversion

High-level logic should depend on an abstraction, not a concrete low-level detail.
Watch for:
- `new ConcreteThing()` constructed directly inside logic that should instead receive
  an injected port/interface — especially for I/O, external services, or anything a
  test would want to fake.
- A domain/application-layer type importing or referencing an infrastructure-layer
  concrete type directly, instead of depending on an interface the infrastructure
  layer implements.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Principle | Result | Location | Note |
| --------- | ------ | -------- | ---- |
| File size | OVER / CROSSED / clean | `datei` | threshold `N`, or "not applicable" if not a size smell |
| S — Single Responsibility | satisfied / violated | `datei:zeile` | — |
| O — Open/Closed | satisfied / violated | `datei:zeile` | — |
| L — Liskov Substitution | satisfied / violated | `datei:zeile` | — |
| I — Interface Segregation | satisfied / violated | `datei:zeile` | — |
| D — Dependency Inversion | satisfied / violated | `datei:zeile` | — |

Only list rows with something to say in detail; a principle the diff doesn't touch at
all can be one line ("not applicable — no changes to class/interface shape"). Don't pad
the report with a full explanation for every satisfied row. End with:

```
Findings: <n>
```

`<n>` = count of `violated` rows (the file-size row counts only if it's an unwaived
`OVER`/`CROSSED` that the review judged a genuine SRP smell, not a bare size flag).
