---
name: ubiquitous-language-doc
description: Generate (or refresh) a CONTEXT.md Ubiquitous-Language glossary derived from the actual code, wire it into CLAUDE.md, then quality-gate the glossary by reviewing it as if it were source code and fixing every finding. Use when the user wants a "Ubiquitous-Language-Dokumentation für dieses Repository", to "create a CONTEXT.md / ubiquitous language glossary from the code", "document the domain language of this repo", "Domänensprache dokumentieren", or just says "ubiquitous language doc".
arguments: Optional. The scope to document (which package/module/library) and optionally the source file(s) to derive terms from (e.g. `src/results/results.py`). If omitted, infer from the repo root or ask via AskUserQuestion.
---

# Ubiquitous-Language Doc

Generate — or refresh — a **CONTEXT.md** that captures the repository's Ubiquitous
Language: the binding domain terms, their precise definitions, and deliberate
boundaries (what each term does *not* mean). Then wire it into CLAUDE.md and
quality-gate it by reviewing the glossary as if it were source code.

This is an **orchestration** skill. Its one job is to coordinate that playbook;
the steps below — generate, gate (fidelity + structure), fix — only make sense run
together, so coordinating them *is* the single job.

```
derive terms from CODE ─► write/refresh CONTEXT.md ─► wire into CLAUDE.md ─► gate the glossary ─► fix findings
        │                          │                                              │
   the target source        grill-with-docs CONTEXT format     docs-code-consistency (fidelity) + thermo-nuclear (structure)
```

**Distinct from its neighbours — do not absorb them.** `init` writes CLAUDE.md;
`grill-with-docs` *maintains* CONTEXT.md/ADRs mid-planning; `deepen-module`
records design decisions. None of them generates a glossary from scratch off the
codebase. This skill is the from-scratch **generator / refresher** of that
glossary — and it reuses their CONTEXT.md format rather than inventing a rival one.

## The iron rule — DERIVE, don't invent

**Every term, definition, invariant, and *Avoid* entry MUST come from the actual
code (and from the user's confirmations) — never from imagination, and never from
the bundled example.**

- If the code does not establish a term, it does not go in the glossary. Silence
  in the code → silence in the doc.
- If a term is ambiguous in the code, **ask** (AskUserQuestion or a direct
  question) — do not guess a definition.
- The bundled example in `assets/` is a **structure & tone model only**. Its
  domain (ordering / Order / Cancellation) is generic on purpose so it can never
  be mistaken for real content. Nothing from it may leak into the output.
- Quote real names and call sites from the target (`unwrap`, `Result`, `Ok`,
  `src/results/results.py`, …). A glossary you could have written without opening
  the code is a failed run.

This is the same fact-discipline the user's other skills enforce (`photo-interpreter`,
`refine-prompt`): only what is actually there, and flag uncertainty instead of
inventing.

## Reuse, don't duplicate

This skill's unique contribution is **generating the glossary from code** plus the
**review loop that gates it**. Everything else is borrowed. The wiring paths it
depends on — the CONTEXT-FORMAT.md cross-reference (`context_format_ref`), the
bundled skeleton (`template_path`), and the CLAUDE.md heading to wire into
(`claude_md_heading`) — live once in `config.json`; read them from there rather
than hard-coding the literals.

- **CONTEXT.md format** → the canonical shape lives at the path in config.json
  `context_format_ref` (the `grill-with-docs` CONTEXT-FORMAT.md), which
  `grill-with-docs` and `deepen-module` already read and write. Reuse it; the
  bundled skeleton in `assets/` is that shape pre-filled with slots. Do **not**
  create a second, conflicting CONTEXT.md convention.
- **Fidelity gate (enforces the iron rule objectively)** → reuse the
  `docs-code-consistency` skill. It returns an objective **PASS/FAIL** that the
  CONTEXT.md matches the code — the "derive, don't invent" guarantee made testable
  instead of left to discipline — and it checks CONTEXT.md against the code the same
  way `grill-with-docs`/`deepen-module` maintain it. It is a *checker, not a fixer*:
  its located drift report feeds the fix step.
- **Structural review** → reuse the `thermo-nuclear-code-quality-review` skill,
  applying its criteria **analogously to prose**. That skill is C#/.NET-oriented;
  you are **not** running a literal code review — you are transferring its
  structure-first criteria to a glossary (see the mapping table below). Say so
  when you invoke it.

## Process

### 1. Resolve scope, source, and language

Read scope and source file(s) from `arguments`. If scope is missing **and** can't
be inferred from the repo root (single obvious package/library), ask with
**AskUserQuestion** (`header: "Scope"`, options = the candidate
packages/modules + "Whole repo"). Identify the source file(s) the core terms come
from — these are the spine of step 2.

Detect the **language**: if a CONTEXT.md, README, or the request is in German, the
glossary is German; else English. Preserve it throughout. (`## Language` is
conventionally left in English even in a German doc — match an existing CONTEXT.md
if one is present.)

**Generate vs. refresh** — check for an existing CONTEXT.md (or `CONTEXT-MAP.md`
for multi-context repos, per CONTEXT-FORMAT.md):

- **None** → generate from scratch.
- **Exists** → **refresh mode**: read it first and *reconcile*, don't overwrite.
  Add newly-found terms, rename where the code renamed, sharpen stale definitions,
  flag terms the code no longer supports — but preserve human-authored prose and
  any `_Avoid_` lines that still hold.

### 2. Derive the terms from the code

Read the source(s) and pull the **core domain terms** the code actually
establishes: the central types, their constructors/helpers, the operations on
them, cross-conversions, and the error hierarchy. (For the `results` library, that
is e.g. `Result`/`Ok`/`Err`, `Option`/`Some`/`Null`, `unwrap`/`map`,
cross-conversions, constructor helpers, the error hierarchy.) For each, capture:
its precise definition, the invariants the code enforces, and the misleading
synonyms to avoid. Skip general programming concepts that aren't specific to this
domain (per CONTEXT-FORMAT.md's rules).

Where the code is ambiguous, list the open question and resolve it with the user
before writing — never paper over it with a guess.

### 3. Write / refresh CONTEXT.md

Fill the bundled skeleton — read it, don't regenerate the scaffold inline. Its
path is config.json `template_path`, resolved against this skill's base dir:

```
<this-skill's-base-dir>/<template_path>
```

Write CONTEXT.md at the repo root (or the per-context location for multi-context
repos). Required sections:

- **Intro** — what the library/context *is*, and an explicit **scope** line: what
  it deliberately is **NOT**.
- **`## Language`** — one entry per core term: precise definition (what it *is*,
  not what it does), the invariants, and an **`_Avoid_:`** line of misleading or
  false synonyms.
- **Example dialogue** (`## Beispieldialog` / `## Example dialogue`) — a
  developer ⇄ domain-expert exchange that resolves a typical misunderstanding at a
  concrete call site in this codebase.

### 4. Wire it into CLAUDE.md

In CLAUDE.md's section named by config.json `claude_md_heading`, add a short
pointer to CONTEXT.md as the source of the Ubiquitous Language — stating **when to
consult it** (before naming domain concepts / writing domain code) and **when to
maintain it** (when a term is added, renamed, or sharpened). Keep it to a couple of
lines; don't duplicate the glossary into CLAUDE.md. If there is no such section,
add the pointer under the nearest architecture/overview heading.

### 5. Gate the glossary

Run two complementary gates over the freshly written CONTEXT.md.

**5a. Fidelity gate — `docs-code-consistency` (this is how the iron rule is
enforced).** Invoke it scoped to CONTEXT.md against the same code you derived from.
It returns an objective **PASS/FAIL** plus a located drift report: every term that
names a type/function/flag the code no longer has, every definition that
contradicts the code, every public surface the glossary claims but the code lacks.
A FAIL means the glossary invented or drifted from the code — the one thing this
skill must never ship. Don't rely on prose discipline alone when an objective
backstop exists.

**5b. Structural review — `thermo-nuclear-code-quality-review` (analogously).**
Invoke it over CONTEXT.md, **telling it explicitly** that this is prose reviewed as
code, not a C# diff — treat each term entry as a "definition/function" and each
cross-reference as a "call/coupling", and transfer its criteria like so:

| Code criterion (thermo-nuclear) | Glossary analogue |
| --- | --- |
| **DRY** — duplicated logic | A term defined twice, or two entries whose definitions overlap / restate each other. Merge to one canonical entry. |
| **Spaghetti** — tangled control flow | Cross-references between entries that are circular or tangled ("see X" → "see Y" → "see X"). Straighten into a clear hierarchy. |
| **Oversized** — a type doing too much | One entry that mixes several distinct terms into a single definition. Split into one entry per concept. |

Also carry the format rules from CONTEXT-FORMAT.md as review checks: every term
has an `_Avoid_` line, definitions say what it *is* (not what it does), and no
general programming concepts crept in.

### 6. Fix every finding

Both gates are *checkers*, so applying the fixes to CONTEXT.md is this skill's job.
Fix **fidelity drift first** (it is the iron-rule violation), then the structural
findings. If a finding exposes a term whose correct definition is genuinely unclear
from the code, ask rather than guess (iron rule). Re-run the gates after substantial
fixes; the skill is done when the fidelity gate is **PASS** and the structural
review is clean.

## Report format

End with a short summary, skipping nothing material:

| Item | Result |
| --- | --- |
| Mode | generated / refreshed |
| Scope & source | `<package>` from `<file(s)>` |
| Terms | `<n>` (e.g. `+3 / renamed 1` in refresh mode) |
| CLAUDE.md | pointer added under the `claude_md_heading` section |
| Fidelity gate | `docs-code-consistency`: PASS (drift items fixed: `<n>`) |
| Structural review | thermo-nuclear: `<n>` findings fixed / clean |
| Open questions | terms the code couldn't settle, awaiting the user |

Close with the single highest-value next action (e.g. "glossary clean; confirm
whether `Null` and `None` are the same term before I finalise that entry").
Don't pad the report with steps that passed cleanly.
