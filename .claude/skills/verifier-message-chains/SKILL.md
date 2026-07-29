---
name: verifier-message-chains
description: Review a diff/branch for navigation chains where a client walks through a series of objects (a.B().C().D()) to reach what it actually needs, coupling it to the whole intermediate structure. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Message Chains code smell — "message chains check", "navigiert der Aufrufer durch eine ganze Kette von Objekten", "is this a train wreck / law of demeter violation".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Message Chains Check

Refactoring.guru's smell: "a client requests another object, that object requests yet
another one, and so on... the client is dependent on navigation along the class
structure." Recognizable at a glance as a `a.B().C().D()` or `a.B.C.D` chain in the
diff.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A chain of 3+ calls/property accesses reaching through intermediate objects** to
  get to a final value (`order.customer().address().city().name()`) → **Hide Delegate**:
  expose the needed value directly on the object the client already holds, so the client
  stops knowing the path through the middle.
- **The same chain repeated at more than one call site** — a stronger version of the
  same finding, since any change to the intermediate structure now breaks multiple
  places → **Extract Method** to name the traversal once, then **Move Method** it onto
  the root object.
- **A chain that reaches through a layer boundary the codebase otherwise keeps closed**
  — a caller in an outer layer navigating into a nested inner-layer shape instead of
  consuming the flat result the boundary hands it — flag regardless of chain length,
  since it breaks the boundary as well as coupling to the path.
- **Distinguish from a fluent API used as intended** — a collection/query pipeline, a
  builder's own chained configuration calls — that's an accepted idiom, not this smell;
  the finding is specifically about navigating a *structural/ownership* graph to reach
  data, not method chaining in general.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Chain | Reaches | Location | Fix |
| ----- | ------- | -------- | --- |
| `order.customer().address().city().name()` | A nested inner-layer field reached from an outer layer | `datei:zeile` | Hide Delegate: `order.customerCityName()` (or flatten the value at the boundary) |

Only list rows for genuine structural navigation chains, not accepted fluent/pipeline
idioms. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
