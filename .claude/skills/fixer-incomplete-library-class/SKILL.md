---
name: fixer-incomplete-library-class
description: Apply the fix for the Incomplete Library Class code smell — centralize a workaround for a library/framework gap instead of leaving it scattered inline. Use when a `verifier-incomplete-library-class` finding needs remediating, or directly asked to fix it — "fix this library gap workaround", "diese Bibliotheksluecke zentralisieren", "centralize this workaround".
arguments: Optional. What to fix — a `verifier-incomplete-library-class` report/finding to act on directly, or a file/dir path, PR number, or diff range to locate-and-fix. Defaults to the current branch's changes against its merge-base with the default branch (same as the paired check).
---

# Incomplete Library Class Fixer

Applies refactoring.guru's fix for this smell: libraries "stop meeting user
needs... changing the library is often impossible since the library is
read-only." Paired with `verifier-incomplete-library-class`, which
distinguishes a genuinely duplicated/misplaced workaround from an accepted
single-site adaptation; this skill applies the fix once a genuine instance
is found.

## Process & report format

Follow `_shared/fixer-contract.md` for input resolution, process, and report
format — not restated here.

## Refactorings to apply

- **The same missing-method workaround duplicated at more than one call
  site** → **Introduce Foreign Method**: one free helper taking the
  library instance as its first parameter, called from every site instead of
  repeating the workaround.
- **A recurring family of workarounds around the same library type** →
  **Introduce Local Extension**: a small wrapper type (subclass, extension,
  or whatever the language offers) owning all of them in one place.
- **A workaround embedded in code that is supposed to be free of that
  library** — patched into business/domain logic rather than at the boundary
  layer that already owns the dependency → move it to that boundary,
  regardless of whether it was duplicated. Follow the layering the
  surrounding code actually exhibits; don't impose one it doesn't have.

Leave a single, clean, single-site adaptation alone — that's the accepted
minimal fix, not a finding to consolidate.
