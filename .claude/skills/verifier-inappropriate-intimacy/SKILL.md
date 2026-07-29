---
name: verifier-inappropriate-intimacy
description: Review a diff/branch for two classes that reach into each other's internal fields/methods, making them too interdependent to change, reuse, or test in isolation. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Inappropriate Intimacy code smell — "inappropriate intimacy check", "greifen sich diese zwei Klassen zu sehr in die Interna", "are these two classes too tightly coupled to each other's internals".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Inappropriate Intimacy Check

Refactoring.guru's smell: "one class uses the internal fields and methods of another
class." Distinct from `verifier-feature-envy` (a method leaning on *one* collaborator's
data) in that this is a **mutual, structural** dependency between two classes on each
other's internals, not a single misplaced method.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **Two classes reading/writing each other's non-public state** — through whatever
  visibility escape the language offers (a package/module-private back door, a
  friend/internal grant, reflection, a naming convention the other side ignores) rather
  than going through a public contract → **Move Method** / **Move Field** to put the
  shared behavior in one place, or **Extract Class** for the part they both need.
- **A bidirectional association where either side can mutate the other freely** →
  **Change Bidirectional Association to Unidirectional** so only one side owns the
  relationship.
- **A class exposing internals just so a sibling class can delegate through it** →
  **Hide Delegate** to remove the need for the sibling to know the internal shape at
  all.
- **A subclass reaching past its own contract into a sibling's private structure**
  where a cleaner relationship would be composition → **Replace Delegation with
  Inheritance** (or the reverse, depending on which direction actually fits).
- **A dependency that only compiles because two modules share internal visibility** —
  it works today solely because a boundary was opened, and it silently breaks the moment
  that boundary is tightened. A strong finding, not a style nit.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Class A | Class B | Intimacy found | Location | Fix |
| ------- | ------- | ---------------- | -------- | --- |
| `WindowingHandler` | `Auswertungsfenster` | Reads `Auswertungsfenster`'s internal buchungen list directly | `datei:zeile` | Hide Delegate: expose a public method instead of the internal collection |

Only list rows with genuine reach into non-public state or a tightly bidirectional
coupling — a normal public-API collaboration between two classes isn't a finding.
End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
