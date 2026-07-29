# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

A collection of **Claude Code agent skills**. Each skill is a directory whose name is the skill's kebab-case name, containing a `SKILL.md`. There is no build, lint, or test step — skills are markdown documents loaded by the Claude Code harness, so "running" a skill means invoking it inside Claude Code, not executing a command here.

The skills live directly in `.claude/skills/` so this repo **loads its own skills as project skills** — you can dogfood them while working here. Claude Code discovers project skills only at `.claude/skills/<name>/SKILL.md` (the `skills/` segment is required — a skill placed directly in `.claude/<name>/` will *not* load). After adding, moving, or renaming a skill, run `/reload-skills`; then invoke it with `/<skill-name>` (e.g. `/skill-audit`, `/skill-tune-up`).

<!-- BEGIN GENERATED: sync-skill-index -->
```text
.claude/skills/
  apply-validator-findings/                               SKILL.md
  architecture-adr-check/                                 SKILL.md  scripts/  config.json
  audit-to-issues/                                        SKILL.md
  auto-commit/                                            SKILL.md  scripts/  config.json
  build-verifier/                                         SKILL.md  scripts/  assets/
  compress-prompt/                                        SKILL.md  scripts/  assets/  config.json
  deepen-module/                                          SKILL.md
  design-pattern-fit-check/                               SKILL.md
  docs-code-consistency/                                  SKILL.md  scripts/  config.json
  done/                                                   SKILL.md  assets/
  fixer-alternative-classes-with-different-interfaces/    SKILL.md
  fixer-comments/                                         SKILL.md
  fixer-data-class/                                       SKILL.md
  fixer-data-clumps/                                      SKILL.md
  fixer-dead-code/                                        SKILL.md
  fixer-divergent-change/                                 SKILL.md
  fixer-duplicate-code/                                   SKILL.md
  fixer-feature-envy/                                     SKILL.md
  fixer-inappropriate-intimacy/                           SKILL.md
  fixer-incomplete-library-class/                         SKILL.md
  fixer-large-class/                                      SKILL.md
  fixer-lazy-class/                                       SKILL.md
  fixer-long-method/                                      SKILL.md
  fixer-long-parameter-list/                              SKILL.md
  fixer-message-chains/                                   SKILL.md
  fixer-middle-man/                                       SKILL.md
  fixer-parallel-inheritance-hierarchies/                 SKILL.md
  fixer-primitive-obsession/                              SKILL.md
  fixer-refused-bequest/                                  SKILL.md
  fixer-shotgun-surgery/                                  SKILL.md
  fixer-speculative-generality/                           SKILL.md
  fixer-switch-statements/                                SKILL.md
  fixer-temporary-field/                                  SKILL.md
  graphify-windows/                                       SKILL.md
  grill-with-docs/                                        SKILL.md
  handoff/                                                SKILL.md  assets/
  illegal-state-check/                                    SKILL.md
  improve-codebase-architecture/                          SKILL.md  assets/  config.json
  issue-close/                                            SKILL.md  config.json
  issue-status/                                           SKILL.md  scripts/  config.json
  issues-to-prs/                                          SKILL.md  assets/  config.json
  language-idiom-check/                                   SKILL.md
  lint-and-format-check/                                  SKILL.md  scripts/  config.json
  multi-agent-thermo-nuclear-review/                      SKILL.md  scripts/  assets/  config.json
  propose-skills/                                         SKILL.md  scripts/
  prototype/                                              SKILL.md
  qa-check/                                               SKILL.md  scripts/  config.json
  refine-prompt/                                          SKILL.md  assets/
  reflect/                                                SKILL.md  scripts/  assets/  config.json
  review-against-rules/                                   SKILL.md  assets/  config.json
  run-tests/                                              SKILL.md  scripts/  config.json
  skill-audit/                                            SKILL.md  scripts/
  skill-tune-up/                                          SKILL.md  scripts/
  solid-principles-check/                                 SKILL.md  scripts/  config.json
  strategic-compact/                                      SKILL.md  scripts/  config.json
  sync-skill-index/                                       SKILL.md  scripts/  assets/  config.json
  tdd/                                                    SKILL.md
  thermo-nuclear-code-quality-review/                     SKILL.md  scripts/  config.json
  to-issues/                                              SKILL.md  scripts/  assets/  config.json
  to-prd/                                                 SKILL.md  assets/
  token-budget-audit/                                     SKILL.md  scripts/  assets/  config.json
  ubiquitous-language-doc/                                SKILL.md  assets/  config.json
  validate-fix-loop/                                      SKILL.md  assets/  config.json
  verbessere-text/                                        SKILL.md  scripts/
  verifier-alternative-classes-with-different-interfaces/ SKILL.md
  verifier-audit/                                         SKILL.md  scripts/
  verifier-comments/                                      SKILL.md
  verifier-data-class/                                    SKILL.md
  verifier-data-clumps/                                   SKILL.md
  verifier-dead-code/                                     SKILL.md
  verifier-divergent-change/                              SKILL.md
  verifier-duplicate-code/                                SKILL.md
  verifier-feature-envy/                                  SKILL.md
  verifier-inappropriate-intimacy/                        SKILL.md
  verifier-incomplete-library-class/                      SKILL.md
  verifier-large-class/                                   SKILL.md  scripts/  config.json
  verifier-lazy-class/                                    SKILL.md
  verifier-long-method/                                   SKILL.md  scripts/  config.json
  verifier-long-parameter-list/                           SKILL.md  scripts/  config.json
  verifier-message-chains/                                SKILL.md
  verifier-middle-man/                                    SKILL.md
  verifier-parallel-inheritance-hierarchies/              SKILL.md
  verifier-primitive-obsession/                           SKILL.md
  verifier-refused-bequest/                               SKILL.md
  verifier-shotgun-surgery/                               SKILL.md
  verifier-speculative-generality/                        SKILL.md
  verifier-switch-statements/                             SKILL.md
  verifier-temporary-field/                               SKILL.md
  verify-issue-breakdown/                                 SKILL.md  scripts/  assets/
  worktree-entfernen/                                     SKILL.md  scripts/  config.json
  worktree-erstellen/                                     SKILL.md  scripts/  config.json
CLAUDE.md
```

Skills are grouped into the four buckets (see Design Contract below). Current inventory by bucket:

**Utility** — one small reusable thing, the same way every time:

- `apply-validator-findings` — takes the findings already produced by a set of quality validators and fixes every one, applying each finding's own located remedy — never re-reviewing or inventing new scope.
- `build-verifier` — scaffolds a new Verification-bucket skill from a description; fills a bundled `assets/` template.
- `compress-prompt` — rewrites a prompt or text to use fewer tokens while preserving intent, applying a fixed set of token-reduction techniques and reporting a before/after token estimate.
- `fixer-alternative-classes-with-different-interfaces` — aligns two types doing the same job under different method names/signatures — Rename Method/Extract Superclass — paired with `verifier-alternative-classes-with-different-interfaces`.
- `fixer-comments` — replaces a WHAT-comment with a name that carries the meaning (Extract Method/Rename/Introduce Assertion), leaving genuine WHY comments untouched — paired with `verifier-comments`.
- `fixer-data-class` — moves behavior into an anemic domain type (Move Method/Encapsulate Field/Collection) — paired with `verifier-data-class`.
- `fixer-data-clumps` — extracts a recurring group of variables/parameters into its own Value Object (Extract Class + Introduce Parameter Object) — paired with `verifier-data-clumps`.
- `fixer-dead-code` — deletes a variable/parameter/field/method/type with zero remaining references — paired with `verifier-dead-code`.
- `fixer-divergent-change` — splits a type that changes for many unrelated reasons into one type per concern (Extract Class/Superclass/Subclass) — paired with `verifier-divergent-change`.
- `fixer-duplicate-code` — replaces near-identical copy-pasted fragments with one shared implementation (Extract Method/Class, Substitute Algorithm) — paired with `verifier-duplicate-code`.
- `fixer-feature-envy` — moves a method to the type whose data it actually operates on (Move Method) — paired with `verifier-feature-envy`.
- `fixer-inappropriate-intimacy` — separates two types reaching into each other's internals (Hide Delegate/Move Method/Change Bidirectional Association) — paired with `verifier-inappropriate-intimacy`.
- `fixer-incomplete-library-class` — centralizes a scattered library/crate-gap workaround (Introduce Foreign Method/Local Extension) — paired with `verifier-incomplete-library-class`.
- `fixer-large-class` — splits a type whose fields/methods cluster into more than one natural group (Extract Class/Subclass/Interface) — paired with `verifier-large-class`.
- `fixer-lazy-class` — inlines or collapses a type that no longer earns its own existence (Inline Class/Collapse Hierarchy) — paired with `verifier-lazy-class`.
- `fixer-long-method` — extracts cohesive pieces out of a method mixing several responsibilities/abstraction levels (Extract Method, Decompose Conditional, Replace Method with Method Object) — paired with `verifier-long-method`.
- `fixer-long-parameter-list` — collapses excess parameters into an object or a derivable lookup (Introduce Parameter Object/Preserve Whole Object) — paired with `verifier-long-parameter-list`.
- `fixer-message-chains` — hides a navigation chain behind a method on the object the client already holds (Hide Delegate) — paired with `verifier-message-chains`.
- `fixer-middle-man` — removes a type whose methods do nothing but forward to another (Remove Middle Man), preserving intentional port/adapter seams — paired with `verifier-middle-man`.
- `fixer-parallel-inheritance-hierarchies` — merges two hierarchies extended in lockstep into one (Move Method/Field) — paired with `verifier-parallel-inheritance-hierarchies`.
- `fixer-primitive-obsession` — replaces a raw primitive standing in for a domain concept with a newtype validating on construction (Replace Data Value with Object, Replace Type Code) — paired with `verifier-primitive-obsession`.
- `fixer-refused-bequest` — replaces an ill-fitting inheritance relationship with delegation or a narrower base (Replace Inheritance with Delegation, Extract Superclass) — paired with `verifier-refused-bequest`.
- `fixer-shotgun-surgery` — consolidates one conceptual change scattered across many files into a single place (Move Method/Field, Inline Class) — paired with `verifier-shotgun-surgery`.
- `fixer-speculative-generality` — removes an unused just-in-case abstraction (Inline Method/Class, Collapse Hierarchy) — paired with `verifier-speculative-generality`.
- `fixer-switch-statements` — moves a discriminant matched on in several places onto the type's own closed sum (Replace Type Code with Subclasses/State-Strategy) — paired with `verifier-switch-statements`.
- `fixer-temporary-field` — extracts a field only meaningful during one algorithm into its own object (Extract Class, Replace Method with Method Object) — paired with `verifier-temporary-field`.
- `handoff` — compacts the current conversation into a handoff document for another agent.
- `issue-close` — closes one issue — sets status: closed in its frontmatter and prepends a completion note under ## Erledigt in PROGRESS.md, updating the Stand: line.
- `issue-status` — scans docs/issues/ for frontmatter status fields and prints a grouped status table (blocked / open / closed / …) to chat — read-only, no files written.
- `refine-prompt` — improves/hardens a draft prompt or task template and returns the rewritten text + a changelog — never executing what the prompt asks.
- `reflect` — end-of-session experience extraction; persists structured experience files with decay metadata.
- `run-tests` — runs the project's test suite via the one canonical invocation declared in its config.json, with a config-driven filter mode, and surfaces the pass/fail result.
- `strategic-compact` — suggests manual context compaction at logical intervals to preserve context.
- `sync-skill-index` — regenerates the skill-inventory region of CLAUDE.md from the filesystem + a bucket map.
- `to-prd` — converts current conversation context into a PRD and publishes it to the issue tracker.
- `verbessere-text` — improves a short text or keywords into three tone variants (AskUserQuestion pick, source language kept), normalized dash-free by a bundled `scripts/strip_dashes.py`.
- `worktree-entfernen` — tears down a worktree only when it is final in main (branch merged) and its tree is clean — junction-safe removal, deletes the merged branch; `--force` to override.
- `worktree-erstellen` — creates a git worktree under `.claude/worktrees/` and makes the local-only context (`.claude/`, `CLAUDE.md`, `CONTEXT.md`) available inside it — platform-appropriate symlink/junction/hardlink, idempotent.

**Verification** — checks the quality of a final output:

- `architecture-adr-check` — checks code changes against this repo's architecture-decision docs and the issue they implement (paths via `config.json`, missing config = `CONFIG ERROR` not a silent pass) — invariants honored, acceptance criteria met — returning PASS/FAIL plus a `Findings: <n>` count.
- `design-pattern-fit-check` — reviews a diff for shapeless/procedural code where a named GoF/architectural pattern (Strategy, State, Factory, Builder, Decorator, Observer, Command, Chain of Responsibility, Template Method, Adapter, Repository) would express it more cleanly — requires naming the concrete pattern and sketching the seams, not just "add an abstraction" — returning BLOCK/APPROVE plus a `Findings: <n>` count.
- `docs-code-consistency` — checks docs (docstrings, README, docs/ADRs/CONTEXT.md) against the current code and returns PASS/FAIL plus an itemized, located drift report — checker, not fixer.
- `illegal-state-check` — reviews a diff's data/type model for representable-but-invalid states — a flag/enum plus optional fields only valid in some combinations, an unconstrained primitive standing in for a constrained domain, a mutable object with violable invariants — and pushes to make the bad state unrepresentable, not merely checked. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `improve-codebase-architecture` — reviews a codebase for deepening opportunities (shallow modules worth collapsing).
- `language-idiom-check` — reviews a diff for missed idioms of the language actually in use (pattern matching, records, collection pipelines, destructuring, ...) and for imperative code that should be declarative (pattern matching over cascades, immutable values, pure functions). Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `lint-and-format-check` — runs this repo's configured `linter`/`formatter` (command + args from `config.json`, any language) — unconfigured proposes a pair by detected language and stops as `CONFIG ERROR` rather than guessing — returning PASS/FAIL plus a `Findings: <n>` count.
- `propose-skills` — scans chat history for repeated manual tasks that should be skills and proposes each (name, bucket, one job), skipping what existing skills cover.
- `qa-check` — runs the test suite via `run-tests`, and — each independently toggleable via `config.json` — flags changed production projects whose co-located specs sibling didn't change, and agent-judges every feature's test facade against Fowler's Test-API pattern (script only enumerates candidates).
- `review-against-rules` — reviews a diff/branch against this repo's configured coding-standard dirs and, if configured, a reference implementation, via the `senior-code-reviewer` subagent — required config missing = `CONFIG ERROR` not a silent pass — returning BLOCK/APPROVE plus a `Findings: <n>` count.
- `skill-audit` — flags existing skills that straddle more than one bucket and recommends a split or trim.
- `skill-tune-up` — audits skills against the five structural levers; flags mechanical weaknesses.
- `solid-principles-check` — reviews a diff against the five SOLID principles (SRP/OCP/LSP/ISP/DIP), each with its own smell checklist, plus a mechanical file-size pre-check (oversized files are a classic SRP smell). Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `thermo-nuclear-code-quality-review` — extremely strict, language-agnostic maintainability review — abstraction quality, oversized files, spaghetti conditions, unrepresentable invalid states, declarative style.
- `token-budget-audit` — audits persistent-context artifacts (CLAUDE.md, SKILL.md files, memory) against token-budget heuristics and returns PASS/FAIL plus a located, itemized report.
- `verifier-alternative-classes-with-different-interfaces` — reviews a diff for two types performing the same job under different method names/signatures, blocking interchangeable use. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-audit` — flags skills that should become verifiers or gain an objective Pass/Fail or grade-out-of-10 check.
- `verifier-comments` — reviews a diff for comments explaining WHAT unclear code does instead of a genuinely non-obvious WHY. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-data-class` — reviews a diff for anemic types holding only fields/accessors with no behavior of their own. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-data-clumps` — reviews a diff for the same small group of variables/parameters recurring together across signatures or fields instead of being extracted into a type. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-dead-code` — reviews a diff for variables/parameters/fields/methods/types with zero remaining references. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-divergent-change` — reviews a diff for a single type edited for many unrelated reasons in one change. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-duplicate-code` — reviews a diff for two or more near-identical code fragments that should be shared instead of copy-pasted. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-feature-envy` — reviews a diff for a method that accesses another object's data more than its own. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-inappropriate-intimacy` — reviews a diff for two types reaching into each other's internal fields/methods. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-incomplete-library-class` — reviews a diff for ad-hoc workarounds scattered around a library/crate gap instead of centralizing them. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-large-class` — reviews a diff for a type that accumulated too many fields/methods/unrelated responsibilities. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-lazy-class` — reviews a diff for a type that no longer does enough to justify its own existence. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-long-method` — reviews a diff for methods mixing several responsibilities/abstraction levels in one body. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-long-parameter-list` — reviews a diff for methods/constructors taking more parameters than the configured threshold. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-message-chains` — reviews a diff for navigation chains (`a.b().c().d()`) coupling a client to the whole intermediate object structure. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-middle-man` — reviews a diff for a type whose methods do nothing but delegate to another. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-parallel-inheritance-hierarchies` — reviews a diff for two type hierarchies that must be extended in lockstep. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-primitive-obsession` — reviews a diff for raw primitives standing in for a concept with its own rules instead of a small dedicated type. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-refused-bequest` — reviews a diff for a subtype that only uses some of its parent's inherited members. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-shotgun-surgery` — reviews a diff for one conceptual change requiring small edits scattered across many types/files. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-speculative-generality` — reviews a diff for abstractions built "just in case" for an imagined future need with no current caller. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-switch-statements` — reviews a diff for the same discriminant matched/if-chained on in more than one place instead of using polymorphism/a closed sum type. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verifier-temporary-field` — reviews a diff for a field only meaningfully set during one algorithm/call and empty otherwise. Returns BLOCK/APPROVE plus a `Findings: <n>` count.
- `verify-issue-breakdown` — verifies that a `to-issues` breakdown is a sound tracer-bullet decomposition before publish.

**Data enrichment** — pulls external knowledge in:

- `graphify-windows` — builds and queries a persistent knowledge graph from a codebase — god nodes, community detection, query/path/explain tools; Windows-compatible variant.

**Orchestration** — chains other skills or steps into a multi-step playbook:

- `audit-to-issues` — audits a repo on a chosen axis (over-engineering / illegal-states / pythonic) and turns each finding into a clean, reviewed, deduped issue — delegates to `ponytail-audit` and `to-issues`.
- `auto-commit` — groups a working tree's changes into cohesive Conventional-Commits, previews the plan, then commits each group.
- `deepen-module` — drives a deepening candidate from architecture review to settled, documented design.
- `done` — end-of-session wrapper: runs `/reflect`, prints summary, updates task state.
- `grill-with-docs` — stress-tests a plan against the domain model and updates documentation inline.
- `issues-to-prs` — implements each selected open issue as an independent, parallel git-worktree agent and ships exactly one PR per issue — disjointness-gated, no shared merge.
- `multi-agent-thermo-nuclear-review` — manual-only multi-agent fan-out of `thermo-nuclear-code-quality-review` — per configured lens, diverse finders + one adversarial verifier run in parallel, reconciled against the agent's own reading; delegates the rubric and size-pass to thermo-nuclear by path.
- `prototype` — builds a throwaway prototype to flesh out a design; routes between two branches.
- `tdd` — test-driven development with red-green-refactor loop.
- `to-issues` — breaks a plan/PRD into tracer-bullet issues, gates with `verify-issue-breakdown` before publish.
- `ubiquitous-language-doc` — generates/refreshes a CONTEXT.md Ubiquitous-Language glossary derived from the code, wires it into CLAUDE.md, and quality-gates it by reviewing the glossary as source code.
- `validate-fix-loop` — dispatches the 23 `verifier-<smell>` code-smell checks listed in `config.json` in parallel each iteration, each its own subagent on `config.json`'s `agent_model`, then routes each verifier's findings — via `fixer_map` — to its own `fixer-<smell>` subagent, in waves whose file sets `scripts/plan_iteration.py` proves disjoint (parallel within a wave, sequential across waves). The orchestrator gathers each fixer's input fresh before its wave and runs the test gate itself once per iteration, so fixers only edit; repeats until every verifier reports zero findings, total findings plateau, or `max_iterations` is hit.
<!-- END GENERATED: sync-skill-index -->

When editing this repo you are almost always authoring or refining a skill, so the conventions below *are* the architecture.

## SKILL.md anatomy

YAML frontmatter then a markdown body:

```text
---
name: kebab-case-name        # must match the directory name
description: <what it does>. Use when <trigger phrases>.
arguments: <optional>        # declare inputs passed at invocation time
---

# Title
<body: process, rules, report format>
```

The `description` follows a fixed two-part pattern: a statement of what the skill does, then a `Use when …` clause listing the trigger phrases that should fire it. The harness matches on this, so it carries real weight — write it for dispatch, not just documentation.

A skill may bundle supporting files (the skill-meta skills all reuse one `scripts/inventory.py` for their inventory step — see the shared-script note below; `build-verifier` also bundles an `assets/` template it reads and fills):

- `scripts/` — deterministic logic the skill runs as code instead of re-deriving in prose.
- `assets/` — boilerplate/templates the skill reads and fills.
- `config.json` — values set once and read, rather than re-entered each run.

**Shared scripts.** When several skills need the same deterministic step, the script has one home under `skills/_shared/` and each skill's `scripts/<name>` is a *relative symlink* to it — one source of truth instead of N drifting copies. `inventory.py` works this way (used by `skill-audit`, `skill-tune-up`, `verifier-audit`, `build-verifier`, `propose-skills`). Edit `skills/_shared/inventory.py`, never a symlink. `_shared/` has no `SKILL.md`, so `skill-linker` and `sync-skill-index` ignore it; the symlinks resolve at each skill's real location, so they keep working through the `.claude/skills/` skill symlinks too.

## The design contract enforced by the existing skills

These two vocabularies are the standard any new skill here is expected to meet — `skill-audit` checks the first, `skill-tune-up` the second. Apply them when writing a skill, not just when auditing one.

**Four buckets (from `skill-audit`)** — every skill should fit *cleanly into exactly one*:

1. **Utility** — one small reusable thing, the same way every time.
2. **Verification** — checks the quality of a final output.
3. **Data enrichment** — pulls external data *in*.
4. **Orchestration** — chains other skills into a multi-step playbook (coordinating many steps is its single job — this is not "straddling").

A skill spanning 2+ buckets confuses the harness about when to fire it; split it or trim it.

**Five structural levers (from `skill-tune-up`)** — prefer these over spelling things out in prose:

1. Deterministic logic → a bundled `scripts/` script.
2. Templated output → `assets/`.
3. Re-entered or hard-coded config → `config.json`.
4. Fixed-option setup questions → the `AskUserQuestion` tool.
5. Invocation-time inputs (slug, path, target) → an `arguments` frontmatter field.

## House style for skill bodies

These skills share a structure worth matching in new ones: a short framing, an explicit numbered **Process**, and a **Report format** with literal markdown tables. Reports are blunt and skip anything already fine ("don't pad the report"), and end with the single highest-value action.
