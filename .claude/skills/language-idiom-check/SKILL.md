---
name: language-idiom-check
description: Review a diff/branch for missed language idioms (pattern matching, records/data classes, collection pipelines, destructuring, expression-bodied members, extension methods, string interpolation, and the like — detected per the language actually in use) and for imperative code that should be declarative (pattern matching over cascading type-tests, immutable values over mutation, pure functions over side-effecting steps). Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking idiomatic/declarative style — "ist das idiomatisch geschrieben", "nutzt das moderne Sprachfeatures", "is this declarative enough", "idiom check".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Language Idiom & Declarative Style Check

Two related judgments, both about *how* a language expresses something rather than
*what* it does structurally (that's `design-pattern-fit-check`/`solid-principles-check`)
or whether its data model is sound (`illegal-state-check`):

1. **Idiom use** — is the diff using this language's own current, well-known idioms, or
   reinventing by hand something the language/standard library already expresses
   concisely?
2. **Declarative style** — where a piece of code is fundamentally a transformation or a
   decision over a set of cases, is it written that way (pattern matching, immutable
   values, pure functions), or as imperative mutation and ad hoc branching?

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)"). **Identify the language(s) from the files under
review first** — idiom checks are inherently language-specific, so look up (from what
you already know, or by reading a few existing files in the same codebase for the
idioms it already leans on) what "current and idiomatic" means for that language before
judging the diff against it.

## What counts as a missed idiom

Illustrative, not exhaustive — the actual list of idioms depends on the detected
language and its current version:

- **Pattern matching** over a cascade of type-tests, casts, or `isinstance`/`instanceof`
  checks and an `if`/`else if` chain on the result.
- **Records / data classes / structs with generated equality** over a hand-written class
  with manually implemented constructor, equality, and accessors.
- **Collection pipelines** (map/filter/reduce/fold, LINQ, list/dict comprehensions,
  streams) over a hand-rolled loop with an accumulator variable.
- **Destructuring / tuple unpacking** over manual field-by-field extraction.
- **Expression-bodied members / concise lambda forms** over a multi-line method body
  that does nothing but return one expression.
- **Extension methods/functions** over a free-floating static helper that takes the
  "extended" type as its first parameter.
- **String interpolation** over manual concatenation or positional `.format()`/`printf`
  calls.
- **Null-safety features** (nullable types, optional chaining, elvis/null-coalescing
  operators) over manual null checks that the language's own syntax would collapse.
- Any other construct the language introduced specifically to replace a common
  hand-rolled pattern (check what the codebase's *other*, unchanged files already use
  — an idiom the codebase has clearly adopted elsewhere but the diff ignores is a
  stronger finding than a purely hypothetical one).

## What counts as non-declarative code worth flagging

- Imperative mutation and branching doing the job a match expression over an exhaustive
  set of cases would do more clearly.
- A mutable accumulator built up in a loop where a pure collection pipeline would say
  the same transformation more directly.
- A function whose result depends on more than its inputs (hidden mutation, shared
  state, side effects mixed into what is otherwise a pure computation) where isolating
  the side effect at the edge and keeping the core pure would simplify reasoning about
  it.

This is a preference, not a mandate to contort code: don't push a declarative rewrite
onto something genuinely stateful or inherently imperative (a tight numeric loop, a
staged I/O sequence) just to force the style — flag it only where the declarative form
would clearly read better, not just be different.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Kind | Missed idiom / non-declarative code | Location | Suggested replacement |
| ---- | ------------------------------------ | -------- | ---------------------- |
| Idiom | hand-rolled loop building a list | `datei:zeile` | `items.Select(...).ToList()` (or the language's pipeline equivalent) |
| Declarative | cascading `is`-checks + casts | `datei:zeile` | pattern match / `switch` expression on the discriminator |

Only list rows with a genuine, concrete finding — don't pad with cosmetic style nits the
project's own formatter/linter already enforces (leave those to `lint-and-format-check`).
End with:

```
Findings: <n>
```

`<n>` = count of concrete missed-idiom or non-declarative findings.
