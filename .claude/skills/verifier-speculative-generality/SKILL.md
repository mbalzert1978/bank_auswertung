---
name: verifier-speculative-generality
description: Review a diff/branch for abstractions built "just in case" for an imagined future need — an unused class, method, field, parameter, hook, or extension point with no current caller. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Speculative Generality code smell — "speculative generality check", "wird hier fuer eine hypothetische Zukunft gebaut", "YAGNI check", "is this abstraction actually needed right now".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Speculative Generality Check

Refactoring.guru's smell: code created "just in case" to support anticipated future
features that never materialize. The bar: an abstraction earns its place by serving a
need that exists **now**. A seam with one implementation, a wrapper with one case, or a
knob with one value is ceremony — it costs every reader a detour and buys nothing until
the second case actually shows up.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **An unused class, method, field, or parameter** introduced in this diff with no
  current caller — the generality was speculative if nothing in the current change
  actually needs it → **Remove Parameter** / **Inline Method** / **Inline Class**.
- **An interface/abstract base with exactly one implementation and no second one
  planned** — a seam built ahead of the second need it's meant to serve →
  **Collapse Hierarchy**.
- **A result/outcome wrapper with a single case**, or an error type introduced for a
  failure mode the operation cannot actually have → collapse it and return the
  underlying value directly. A wrapper is worth its ceremony only once there is a second
  case to distinguish.
- **A configuration knob, extension point, or strategy parameter added "so it's
  flexible later"** with only one value ever passed → remove the parameter, hard-code
  the one real behavior until a second case actually exists.
- **Don't flag genuine, currently-used extensibility** — an interface with two or more
  real implementations, or a parameter two or more callers already pass differently, is
  not speculative; the finding requires the generality being unused *right now*.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Abstraction | Why it's speculative | Location | Fix |
| ----------- | ---------------------- | -------- | --- |
| `IReservierungStrategie` | One implementation, no second planned | `datei:zeile` | Collapse Hierarchy — inline the one implementation, drop the interface |

Only list rows for abstractions with zero current second use — an interface with real
multiple implementations is not a finding even if it looks generic. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
