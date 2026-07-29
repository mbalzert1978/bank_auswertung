---
name: verifier-parallel-inheritance-hierarchies
description: Review a diff/branch for two class hierarchies that must be extended in lockstep — adding a subclass to one forces adding a matching subclass to the other, doubling the maintenance cost of every new variant. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Parallel Inheritance Hierarchies code smell — "parallel inheritance hierarchies check", "muss ich bei jeder neuen Subklasse hier auch dort eine anlegen", "do these two hierarchies always grow together".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Parallel Inheritance Hierarchies Check

Refactoring.guru's smell: "whenever you create a subclass for a class, you find
yourself needing to create a subclass for another class." The naming convention
(`XHandler`/`XPresenter`, `YReader`/`YWriter`) usually gives it away before the
duplication does.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A diff adding a new subclass to one hierarchy alongside a same-named/same-shaped
  new subclass in a second hierarchy** — the tell-tale pattern is both PRs' new types
  sharing a discriminant name (`FooHandler` + `FooPresenter`, both keyed off the same
  new case).
- **The second hierarchy's subclass does nothing but mirror/delegate to the first's**
  → **Move Method** / **Move Field** to merge the mirrored behavior into one hierarchy,
  letting the second reference or be instantiated by the first instead of shadowing it
  one-for-one.
- **Distinguish from intended parallel structure** — a deliberate per-feature shape
  (a command with its handler and its response, a model with its migration) repeated
  across features is a convention, not this smell. The finding is specifically an
  *inheritance* hierarchy pair that grows in lockstep with no compression opportunity,
  not a repeated but intentional composition pattern.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Hierarchy A | Hierarchy B | New pair added together | Location | Fix |
| ----------- | ----------- | ------------------------- | -------- | --- |
| `ReportGenerator` subclasses | `ReportFormatter` subclasses | `PdfReportGenerator` + `PdfReportFormatter` | `datei:zeile`, `datei:zeile` | Move the formatting behavior into the generator hierarchy; drop the mirrored formatter hierarchy |

Only list rows with genuine one-for-one subclass mirroring across two hierarchies.
End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
