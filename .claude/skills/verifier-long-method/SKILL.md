---
name: verifier-long-method
description: Review a diff/branch for methods that do too much in one body — mixed levels of abstraction, a wall of code that needs comments to explain its sections, or a length that forces the reader to hold the whole thing in their head at once. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Long Method code smell — "ist die Methode zu lang", "long method smell", "does this method do too much", "method needs splitting up".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Long Method Check

Refactoring.guru's Long Method smell: "any method longer than ten lines should make you
start asking questions." Line count is only a proxy — the real defect is a method that
mixes several levels of abstraction or several responsibilities in one body, so a reader
can't tell what it does without reading all of it. This is narrower than a general
maintainability pass: the finding here is specifically about **method size and cohesion**,
not naming, style, or data shape (see `verifier-data-class`, `verifier-primitive-obsession` for
those).

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

Run the bundled length pass **before** reviewing — the one fully objective signal
here, so don't eyeball it:

```bash
uv run .claude/skills/verifier-long-method/scripts/file_size_check.py [target]
```

It lists every file under review with its old/new line counts and flags any that
exceed (`OVER`) or just crossed (`CROSSED`) the threshold in `config.json`'s
`file_size_warn_lines`. It is deliberately low: a flag here should plausibly mean "one
method is dominating this file", not "this file has many legitimate responsibilities"
(that is `verifier-large-class`'s question, at its own higher threshold). Tune the
number to the codebase at hand — a good starting point is somewhere near its 75th
percentile file length. A flagged file is
a **candidate** — open it and judge using the bullets below; a long file that's
several short, cohesive methods (or one long-but-single-purpose, same-level
sequence) is not a Long Method finding just because the file crossed the threshold.

## What to look for

- **Length as a trigger, not the verdict** — a method past ~10-15 lines is worth opening;
  whether it's a genuine finding depends on the next three bullets.
- **Mixed levels of abstraction** — high-level orchestration ("process the order") and
  low-level detail (string parsing, loop bodies, raw field access) sitting in the same
  method. The fix: **Extract Method** per level, so the outer method reads like a summary.
- **A comment introducing a section** — `// validate inputs`, `// now compute totals` —
  is a free signal that the section is really its own method waiting to be named
  (**Extract Method**, then delete the comment because the name replaces it).
- **Local variables recomputed or threaded through many lines** — a temp that's set once
  and read many lines later is a sign the surrounding logic wants **Replace Temp with
  Query** or its own method.
- **Long, uninterrupted conditional/looping logic** — nested `if`/`for` blocks doing
  distinct jobs → **Decompose Conditional**, or, if the method's real problem is that it
  takes a large, related group of parameters/locals, **Introduce Parameter Object** /
  **Preserve Whole Object** first, then extract.
- **A method too tangled to extract cleanly (shared mutable locals everywhere)** →
  **Replace Method with Method Object** so the locals become fields of a small object
  built just for this one call.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Method | Lines/shape | Location | Fix |
| ------ | ----------- | -------- | --- |
| `OrderProcessor.Process` | ~60 lines, 3 abstraction levels (validate/compute/persist) | `datei:zeile` | Extract Method per level: `Validate`, `ComputeTotals`, `Persist` |

Only list rows for methods that genuinely mix responsibilities or abstraction levels —
don't pad with a merely-long-but-single-purpose method (a long straight-line sequence of
same-level steps is a weaker finding than mixed abstraction; use judgment, not a hard line
count). End with:

```
Findings: <n>
```

`<n>` = count of concrete Long Method findings.
