---
name: verifier-long-parameter-list
description: Review a diff/branch for methods/constructors taking more than three or four parameters, making call sites hard to read and easy to miscall by position. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Long Parameter List code smell — "long parameter list check", "hat die Methode zu viele Parameter", "should these params be an object".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Long Parameter List Check

Refactoring.guru's Long Parameter List smell: more than three or four parameters make a
method hard to understand and easy to call wrong (same-typed parameters swapped by
position). This is the signature-level counterpart to `verifier-data-clumps` (which flags
the *same group* recurring across several signatures) — a single long signature is a
finding here even if it never repeats elsewhere.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

Run the bundled group-size pass **before** reviewing — the one fully objective
signal here, so don't eyeball every signature:

```bash
uv run .claude/skills/verifier-long-parameter-list/scripts/parameter_count_check.py [target]
```

It lists every parenthesized, comma-separated group in the files under review and
flags any with more than `config.json`'s `max_group_items` (4 — refactoring.guru's
own "three or four"). Deliberately language-agnostic: it does not try to recognise
"is this a declaration" (that needs per-language keywords), so it also surfaces
call sites and tuple literals, not just signatures — a flagged group is a
**candidate**, and only the ones that are genuinely a method/constructor signature
belong in the report below.

## What to look for

- **More than three or four parameters** on a method or constructor, especially several
  of the same primitive type in a row (`string`, `string`, `bool`, `bool`) where a
  transposition at the call site would compile but be wrong.
- **A parameter that could be derived from another parameter already passed** — pass the
  richer object instead and let the callee pull what it needs → **Preserve Whole
  Object**.
- **A parameter that's really config/environment, not per-call data** (a flag threaded
  through every layer to reach one distant branch) → **Replace Parameter with Method
  Call** if the callee can look it up itself, or hoist it out of the parameter list
  entirely.
- **A cohesive group of parameters that belongs together** (already flagged once is
  enough — cross-reference `verifier-data-clumps` rather than double-counting) →
  **Introduce Parameter Object**.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Method | Parameter count/shape | Location | Fix |
| ------ | ---------------------- | -------- | --- |
| `transfer(string,string,string,bool,bool)` | 5 params, 2 adjacent booleans | `datei:zeile` | Introduce Parameter Object `TransferRequest`; replace the two booleans with a named option type |

Only list rows past the three/four-parameter threshold — don't flag a two- or
three-parameter method just because it could theoretically be an object. End with:

```
Findings: <n>
```

`<n>` = count of concrete Long Parameter List findings.
