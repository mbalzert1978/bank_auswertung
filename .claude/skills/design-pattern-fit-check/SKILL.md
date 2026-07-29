---
name: design-pattern-fit-check
description: Review a diff/branch for shapeless/procedural code where a named, well-known design pattern (Strategy, State, Factory, Builder, Decorator, Observer, Command, Chain of Responsibility, Template Method, Adapter, Repository, Specification) would express it more cleanly, and requires naming the concrete pattern plus sketching the resulting seams — not just "add an abstraction". Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for missed design-pattern opportunities — "fehlt hier ein Design Pattern", "ist das eigentlich eine Strategy", "does this need a state machine", "missed pattern check".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Design Pattern Fit Check

Judges whether the diff reinvents, by hand and ad hoc, something a well-known design
pattern already names and shapes well. The failure mode to hunt is "slop": code that
works but makes no real design decision — a flat sequence of steps, or a branch-heavy
procedure — where the underlying problem clearly has the shape of a known pattern that
nobody named.

This is deliberately narrower than a general abstraction review
(`thermo-nuclear-code-quality-review`, standalone) or the SOLID/idiom/illegal-state
splits: the finding here is only valid if you can **name the specific pattern** and
sketch the concrete types/functions it decomposes into and what the call site would
read like afterward. "This could use more abstraction" is not a finding; "this is a
Strategy — extract an `IDiscountPolicy` with one implementation per current `if` arm"
is.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)"). Identify the language(s) from the files under
review — every pattern below translates across languages (a "Strategy" is an
interface + implementations in Java/C#, a `Protocol` + functions in Python, a trait
object in Rust, a discriminated union + function map in TypeScript).

## Patterns to check for

For each, the tell-tale smell to look for and the pattern that resolves it:

- **Strategy / State** — a flag or enum field, plus branching on it in several places,
  selecting between behaviors or driving a lifecycle. Extract one implementation per
  case behind a shared interface (Strategy) or a state machine with explicit
  transitions (State) instead of scattering the branch.
- **Factory / Builder** — object construction logic (which fields to set, in what
  order, with what defaults) duplicated or scattered across several call sites instead
  of centralized in one place. A multi-step, order-sensitive construction is a Builder
  candidate; a "pick the right concrete type" decision is a Factory candidate.
- **Decorator** — cross-cutting behavior (logging, retry, caching, auth) bolted
  directly into a class's core logic instead of wrapping the base behavior in a
  composable layer.
- **Observer** — manual polling, or a hand-rolled list of callbacks/listeners invoked
  ad hoc, standing in for a proper publish/subscribe seam.
- **Command** — an action (undoable, queueable, or loggable) represented as a direct
  method call instead of a reified object/closure carrying what to do and how to undo
  it.
- **Chain of Responsibility** — nested `if`/`else if` gatekeeping (permission checks,
  validation steps, middleware-like processing) where each check could be its own
  link, tried in order, instead of one large conditional.
- **Template Method** — the same algorithm skeleton duplicated with only a few steps
  varying between call sites, instead of one shared skeleton with the varying steps as
  overridable/injectable hooks.
- **Adapter** — inline, ad hoc translation of an external shape into an internal one
  scattered at each use site, instead of one adapter type owning the translation.
- **Repository / Specification** — data-access or query-filtering logic duplicated
  across handlers instead of centralized behind a repository interface or a composable
  specification.

Don't force-fit a pattern where the code is already simple enough that a pattern would
add ceremony without earning its keep — a two-branch `if` that will very plausibly
never grow a third case doesn't need a Strategy. The bar is: is the diff *currently*
paying a real, visible cost (duplication, scattered branching, repeated construction
logic) that the pattern would remove.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Pattern | Smell found | Location | Sketch |
| ------- | ----------- | -------- | ------ |
| Strategy | flag-driven branching across 3 call sites | `datei:zeile` | extract `IDiscountPolicy`, one impl per current arm; call site becomes `policy.Apply(order)` |

Only list rows for patterns actually missing and worth adding — don't pad the report
with patterns that don't apply. If nothing rises to a real finding, say so in one line
per pattern class you actively checked, not a row each. End with:

```
Findings: <n>
```

`<n>` = count of concrete, named pattern-fit findings (not "could be cleaner" nits).
