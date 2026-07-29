---
name: verifier-middle-man
description: Review a diff/branch for a class whose methods do nothing but delegate to another class, adding a layer of indirection without adding value. Returns BLOCK/APPROVE plus a `Findings: <n>` count. Use as one leg of a validator loop, or standalone when checking for the Middle Man code smell — "middle man check", "delegiert diese Klasse nur ohne eigenen Mehrwert", "is this class just a pass-through wrapper".
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes against its merge-base with the default branch.
---

# Middle Man Check

Refactoring.guru's smell: "a class performs only one action, delegating work to
another class" — if that's all it does, "why does it exist at all?" The one-technique
treatment (**Remove Middle Man**) makes this one of the more mechanically checkable
smells here: does every public method on this type do nothing but forward its call.

## Scope & setup

Scope resolution follows `_shared/validator-contract.md` ("Scope resolution
(diff-scoped validators)").

## What to look for

- **Every public method on a class is a one-line forward to another object**, with no
  added parameter shaping, no error translation, no logic of its own → **Remove Middle
  Man**: have callers depend on the delegate directly, then delete the wrapper.
- **A near-total pass-through with one genuinely useful method** — a weaker finding;
  note it, but the fix may be to keep the class for that one method and remove only the
  pure-forwarding ones, not necessarily delete the whole type.
- **Distinguish from an intentional architectural seam** — a port/adapter boundary, a
  facade that isolates a dependency so it can be swapped or faked, an anti-corruption
  layer translating a foreign model, or any indirection that exists to keep a dependency
  pointing the right way. These delegate *by design*: the indirection itself is the
  value, so they are **not** findings even when every method forwards. Only flag a
  delegate-only type that adds a hop **without** serving such a purpose — and when in
  doubt about which it is, say so in the note rather than flagging it outright.
- **A `Middle Man` masking a `Feature Envy`/`Inappropriate Intimacy` finding upstream**
  — if the delegate itself is being reached into elsewhere, note that under the
  relevant check instead of duplicating here.

## Report format

**Verdict: BLOCK** or **Verdict: APPROVE**, then:

| Class | Delegates to | Own added value | Location | Fix |
| ----- | -------------- | ------------------ | -------- | --- |
| `ObfuscatorWrapper` | `Obfuscator` | None — every method is a 1:1 forward | `datei:zeile` | Remove Middle Man; callers depend on `Obfuscator` directly |

Only list rows for types with no structural/architectural reason to exist as an
indirection layer — a deliberate port/adapter seam is not a finding. End with:

```
Findings: <n>
```

`<n>` = count of concrete findings.
