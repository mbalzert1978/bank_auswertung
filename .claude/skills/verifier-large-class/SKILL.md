---
name: verifier-large-class
description: Review a diff/branch for a class that has accumulated too many fields, methods, or unrelated responsibilities — the class-level counterpart to Long Method, where "mentally easier to add to an existing class" won out over creating a new one. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Large Class code smell — "ist die Klasse zu gross geworden", "large class smell", "god object check", "does this class do too much".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Large Class Check

Refactoring.guru's Large Class smell: "a class contains many fields/methods/lines of
code." Classes accumulate responsibilities over time because it's easier to bolt a new
method onto an existing class than to create one. This is a **class-shape** check —
low field/method cohesion and unrelated responsibilities in one type — distinct from
a bare line-count trigger and from `verifier-divergent-change` (which asks how many
*reasons to change* the diff reveals): this check looks specifically at whether the
class's own fields/methods cluster into more than one natural group.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

Run the bundled length pass **before** reviewing — the one fully objective signal
here, so don't eyeball it:

```bash
uv run .claude/skills/verifier-large-class/scripts/file_size_check.py [target]
```

It lists every file under review with its old/new line counts and flags any that
exceed (`OVER`) or just crossed (`CROSSED`) the threshold in `config.json`'s
`file_size_warn_lines`. Tune that number to the codebase at hand — it is a trigger for
a closer look, not a verdict. A flagged file is a **candidate**: open it and judge
using the bullets below; a long file that's still one cohesive concept (a big
enumeration/mapping table) is not a Large Class finding just because it crossed the
threshold.

## What to look for

- **Field clusters that are never used together** — some methods touch fields A/B, other
  methods touch fields C/D, and no method touches both groups. That's two classes
  pretending to be one → **Extract Class**.
- **A class doing one job with two variant strategies bolted on as flags/branches** →
  **Extract Subclass** for the variant behavior instead of an `if (mode == X)` sprinkled
  through the class.
- **A class exposing more surface than one collaborator needs** — callers only ever use
  a slice of its public members → **Extract Interface** so each caller depends on the
  slice it actually needs.
- **Parallel data arrays/observed-data duplicated per instance** where a proper object
  per data point would do → **Duplicate Observed Data**.
- **Growth-by-accretion signal**: recently added methods/fields in the diff that don't
  touch the class's existing core state — a fresh sign the class picked up a
  responsibility that belongs elsewhere.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Class | Responsibility clusters found | Location | Fix |
| ----- | ------------------------------ | -------- | --- |
| `TeilbereichService` | Bereich-Lookup (3 methods/2 fields) + IP-Zuteilung (4 methods/3 fields), never overlapping | `datei:zeile` | Extract Class: `TeilbereichLookup` + `FreieIpZuteilung` |

Only list rows for classes with a genuinely separable cluster — a big class that is
still one cohesive concept isn't a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete Large Class findings.
