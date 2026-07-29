---
name: verifier-incomplete-library-class
description: Review a diff/branch for ad-hoc workarounds scattered around a gap in a third-party/library/framework type that can't be modified directly, instead of centralizing the extra behavior in one place. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Incomplete Library Class code smell — "incomplete library class check", "wird eine Bibliothekslücke an mehreren Stellen einzeln umschifft", "should this library gap be centralized instead of worked around inline".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Incomplete Library Class Check

Refactoring.guru's smell: "libraries stop meeting user needs... changing the library is
often impossible since the library is read-only." Unlike most smells here, the defect
isn't in this codebase's own type design — it's in how the diff *compensates* for a gap
in a type it doesn't own (a standard-library type, a third-party package's type, a
generated client).

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **The same missing-method workaround duplicated at more than one call site** around a
  library type (e.g. re-deriving a value the library's row/record/response type doesn't
  expose directly) → **Introduce Foreign Method**: one free helper that takes the
  library instance as its first parameter, called from every site instead of repeating
  the workaround.
- **A recurring family of workarounds around the same library type** (not just one
  missing method but several) → **Introduce Local Extension**: a small wrapper/subclass
  that owns all of them in one place.
- **A workaround embedded in code that is supposed to be free of that library** — the
  gap gets patched inside business/domain logic instead of at the boundary layer that
  already owns the dependency. Flag as misplaced regardless of whether it's duplicated:
  it leaks the library past a seam the codebase otherwise maintains. (Judge this against
  the layering the surrounding code actually exhibits, not an assumed architecture.)
- **Don't flag one clean, single-site adaptation** — an isolated extension method used
  in exactly one place with no duplication is the accepted minimal fix, not a finding.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Library type/gap | Workaround duplicated at | Location(s) | Fix |
| ------------------ | --------------------------- | ------------ | --- |
| the library's record type has no "cell by header, trimmed" op | Re-implemented inline in 2 places | `datei:zeile`, `datei:zeile` | Introduce Foreign Method `trimmed_cell(record, name)` in one place |

Only list rows with genuine duplication or a misplaced workaround bypassing an existing
port seam — a single clean adaptation isn't a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
