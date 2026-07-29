---
name: deepen-module
description: Drive a chosen deepening candidate from architecture review to a settled, documented design — grills the design tree, explores interfaces with parallel sub-agents, and records decisions in CONTEXT.md/ADRs. Use after an architecture review when the user picks a candidate to pursue, or when they want to design a deepened module, explore interfaces for a refactor, or work through how to consolidate tightly-coupled shallow modules.
arguments: The deepening candidate to drive — the module/package/seam to deepen, as a name or path (usually handed off from `/improve-codebase-architecture`). If omitted, ask which candidate to pursue before starting.
---

# Deepen Module

Take one **deepening candidate** — usually handed off from `/improve-codebase-architecture`, sometimes named directly — and drive it to a settled, documented design. The candidate is *what* to deepen; this skill works out *how*, then captures the decisions.

This is an **orchestration** skill. Its one job is to coordinate the design-and-document follow-through:

- **`/grill-with-docs`** runs the grilling interview and owns every documentation side-effect (`CONTEXT.md` terms, ADRs).
- **[INTERFACE-DESIGN.md](INTERFACE-DESIGN.md)** runs the parallel sub-agent interface exploration ("Design It Twice").
- **[DEEPENING.md](DEEPENING.md)** supplies the architecture-specific spine: dependency categories, seam discipline, and the replace-don't-layer testing strategy.

Don't re-implement the grilling or the doc discipline here — delegate it. This skill adds the deepening-specific design tree and decides when to reach for interface exploration.

## Glossary

Use the architecture vocabulary exactly — **module**, **interface**, **implementation**, **depth**, **deep/shallow**, **seam**, **adapter**, **leverage**, **locality**. Full definitions in [LANGUAGE.md](../improve-codebase-architecture/LANGUAGE.md). Use `CONTEXT.md` vocabulary for the domain. Don't drift into "component," "service," "API," or "boundary."

## Process

### 1. Frame the candidate

Restate the chosen deepening in glossary terms: which shallow modules collapse into one deep module, where the seam lands, and what sits behind it. Then **classify its dependency category** using [DEEPENING.md](DEEPENING.md) — in-process, local-substitutable, remote-but-owned (ports & adapters), or true-external (mock). The category determines how the deepened module is tested across its seam, so it shapes the whole design.

If the candidate came from an architecture review, the deletion test is already done — don't re-litigate whether to deepen. If it was named directly, apply the deletion test from [LANGUAGE.md](../improve-codebase-architecture/LANGUAGE.md) first to confirm there's real complexity to concentrate.

### 2. Grill the design

Hand the design tree to **`/grill-with-docs`** to run as an interview — one question at a time, recommended answer for each, codebase explored instead of asked where possible. `grill-with-docs` owns the glossary challenges and writes `CONTEXT.md`/ADR updates inline as decisions crystallise; don't duplicate that machinery.

Supply it the deepening-specific branches to walk:

- **Seam placement** — where the interface should live, and what stays internal. (One adapter = hypothetical seam; two = real one — see [DEEPENING.md](DEEPENING.md).)
- **Behind the seam** — what behaviour the deep module absorbs from the shallow wrappers.
- **Testing strategy** — driven by the dependency category: stand-in, in-memory adapter, or mock. Replace, don't layer ([DEEPENING.md](DEEPENING.md)).
- **Surviving tests** — which existing tests describe behaviour at the new interface and survive; which were testing past a shallow interface and become waste.

Side-effects ride on `grill-with-docs`:

- Naming the deepened module after a concept not in `CONTEXT.md` → it adds the term.
- Sharpening a fuzzy term mid-conversation → it updates `CONTEXT.md` there.
- User rejects the candidate with a **load-bearing** reason a future explorer would need to avoid re-suggesting it → it offers an ADR. Skip ephemeral ("not worth it now") and self-evident reasons.

### 3. Explore interfaces (when the shape is contested)

When the deepened module's interface is non-obvious, contested, or high-leverage enough to be worth getting right, run the parallel sub-agent exploration in [INTERFACE-DESIGN.md](INTERFACE-DESIGN.md) — 3+ agents, each with a radically different design constraint, presented and compared with an opinionated recommendation. Skip this when the interface is obvious from the grilling.

### 4. Land it

The design is settled when the seam, what's behind it, and the testing strategy are agreed and the relevant `CONTEXT.md`/ADR updates are written (via `grill-with-docs`). State the final shape in glossary terms — interface, what it hides, dependency category, how it's tested — so it's ready to implement.
