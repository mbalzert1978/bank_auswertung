---
name: verifier-dead-code
description: Review a diff/branch for a variable, parameter, field, method, or class that's no longer used, usually left over from a prior refactor. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Dead Code smell — "dead code check", "wird das ueberhaupt noch benutzt", "is this method/field unused now".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Dead Code Check

Refactoring.guru's smell: "a variable, parameter, field, method or class is no longer
used (usually because it's obsolete)." The check is deliberately literal — genuinely
zero remaining references — not a judgment call about whether something *might* still
be useful.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **A method/field/class with zero remaining call sites/references** after the diff —
  search the whole repo, not just the changed file, before flagging → **delete it**
  outright; don't comment it out, don't rename it to `_unused`.
- **A parameter that's accepted but never read in the method body** → **Remove
  Parameter**.
- **A class only ever referenced by other dead code** (a chain of unused types) →
  **Inline Class** / **Collapse Hierarchy** to remove the whole chain at once.
- **A leftover branch/case that can never execute after this diff** (an old code path
  a refactor made unreachable) — flag even though it isn't a whole unused symbol.
- **Don't flag non-public members reachable only from test code** — search the test
  suite before calling anything dead. A member used exclusively by tests is still live;
  whether that indicates a *different* problem is not this check's question.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Symbol | Kind | Location | Fix |
| ------ | ---- | -------- | --- |
| `MacSucheZeileMapper` | class, superseded by the new Shared mapper, zero remaining references | `datei:zeile` | Delete the file |

Only list rows you've confirmed have zero remaining references anywhere in the repo
(including tests) — a symbol that's merely rarely used is not dead code. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
