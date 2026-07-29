---
name: thermo-nuclear-code-quality-review
description: Run an extremely strict, language-agnostic maintainability review of code (or any structured artifact) for abstraction quality, oversized files, spaghetti-condition growth, unrepresentable invalid states, and declarative style. Use this whenever the user asks for a thermo-nuclear code quality review, thermonuclear review, deep code quality audit, or an especially harsh maintainability review of a branch, PR, or diff. Reach for it even when the user just says "tear this PR apart", "be brutal about the structure here", or "is there a cleaner way to write this?" — the point is an ambitious, structure-first review, not gentle nits.
arguments: Optional. What to review — a file/dir path, a PR number, or a base branch / diff range. Defaults to the current branch's changes when omitted.
---

# Thermo-Nuclear Code Quality Review

Use this skill for an unusually strict review focused on implementation quality, maintainability, abstraction quality, and codebase health, in whatever language or artifact the changes happen to be written in.

Above all, this skill should push you to be ambitious about code structure. Don't merely identify local cleanup opportunities. Actively search for "code judo" moves: restructurings that preserve behavior while making the implementation dramatically simpler, smaller, more direct, and more elegant. Modern languages are expressive — the right move is often to let the language, its standard library, and the framework do work that the diff is currently doing by hand.

Identify the language(s) and artifact types of the changes under review from the files themselves, and apply every standard below in that idiom — these rules are about structure, not syntax, so they translate across languages. If the changes aren't in a programming language at all (a Markdown document, a config file, prose), still apply the same lens: duplication, a missing organizing abstraction, an oversized file, and tangled/spaghetti structure are all just as real there, and just as worth flagging.

## Core Prompt

Start from this baseline:

> Perform a deep code quality audit of the current branch's changes. Rethink how to structure / implement the changes to meaningfully improve code quality without impacting behavior. Work to improve abstractions, modularity, reduce spaghetti code, and improve succinctness and legibility. Be ambitious — if there is a clear path to improving the implementation that involves restructuring some of the codebase, go for it. Be extremely thorough and rigorous. Measure twice, cut once.

## Scope & setup

Resolve the target from the invocation `arguments`: a file or directory path, a PR number, or a base branch / diff range. If nothing was passed, review the current branch's changes against its merge-base with the default branch.

Then run the bundled file-size pass **before** reviewing — it's the one fully objective check in this skill, so don't eyeball it:

```bash
uv run .claude/skills/thermo-nuclear-code-quality-review/scripts/changed_files.py [target]
```

It lists every file under review with its old and new line counts and flags any that exceed (`OVER`) — or have just crossed (`CROSSED`) — the size threshold. The threshold lives in `config.json` (`file_size_warn_lines`); the script reads it from there, and so should you, rather than assuming a number.

## Non-Negotiable Additional Standards

Apply the baseline prompt above, plus these explicit review rules.

### Recognize when a *great* abstraction is missing — and design it

Simplification cuts two ways, and this is the half most models get wrong. Deleting indirection is the easy, subtractive win. The harder and more valuable judgment is spotting the place where the code is missing the *right* abstraction — and then actually designing it.

The failure mode to fight is "slop": code that technically works but makes no real design decisions. Long procedural methods that inline three different concerns. A flat sequence of steps where the underlying problem has a clear shape (a pipeline, a state machine, a producer/consumer, a strategy set, a small algebra of operations) that nobody named. When you see that, don't just tidy it — say what the missing abstraction is and sketch it.

What a genuinely good abstraction looks like, and what to reward when you find it (or propose when it's absent):
- It carves the problem at a real joint. Each piece has one reason to change, and the seams correspond to actual concepts in the domain, not to arbitrary line-count splits.
- It makes the call site read like a description of *what* is happening, with the *how* tucked behind a clear contract. The right number of abstractions for a problem like an outbox processor is usually a small handful (think: a message source, a dispatch/handler step, a retry/failure policy, a commit boundary) — each small, each named, each composable. Not one giant method, and not fifteen ceremonial interfaces either.
- It uses the right paradigm for the shape. Most languages are multi-paradigm — lean into a functional style where the problem is a transformation: pure functions, function composition, declarative collection pipelines, immutable data flowing through stages, sum-type/`Result`-style returns instead of control-flow-by-exception. Lean into OO/polymorphism where the problem is genuinely about polymorphic behavior or identity. Picking the fitting paradigm is itself a design decision worth calling out.
- It composes. Small pieces that snap together (a step that takes a step, a policy that wraps a policy) beat one monolith with flags.
- It would feel inevitable to the next reader — they'd think "of course it's structured this way," not "why are there four types here?"

So in the review, alongside "what can we delete," always ask: **is there a missing abstraction here that would turn this slop into something clean?** If yes, name it concretely, describe the few types/functions it decomposes into, show the shape of the call site after, and explain why those are the *right* seams. Be just as demanding here as you are about spaghetti — a PR that ships a working-but-shapeless blob when an elegant 3-4-piece decomposition is right there is a presumptive blocker, not a pass.

The discipline, though: more abstractions are only better when each one earns its keep (see the rule on thin wrappers below). The goal isn't maximum structure — it's the *minimal set of well-chosen abstractions that makes the design feel inevitable*. Reward the PR that found that set. Push the PR that didn't.

### Be ambitious about structural simplification

Don't stop at "this could be a bit cleaner." Look for opportunities to reframe the change so that whole branches, helpers, modes, conditionals, or layers disappear entirely. Prefer the solution that makes the code feel inevitable in hindsight. Assume there is often a code-judo move available: a re-organization that uses the existing architecture more effectively and makes the change dramatically simpler. If you see a path to *delete* complexity rather than rearrange it, push hard for that path.

Common code-judo wins look like:
- Replacing a chain of `if`/`else if` on a type or a discriminator with a match expression and pattern matching (on type, shape, value ranges, and combinations) where the language supports it.
- Modelling a closed set of cases as a sum type / discriminated union — however the language spells it (a sealed class hierarchy, an enum that carries data, a tagged union/variant) — and matching on it exhaustively, instead of passing around an enum plus a bag of optional fields that are only valid in some combinations.
- Letting a declarative collection pipeline (map/filter/reduce/fold) express a transformation instead of a hand-rolled loop with accumulator variables and mutable state.
- Deleting hand-written boilerplate in favor of the language's concise affordances (auto-generated accessors, data-class/record syntax, concise field/property forms).
- Collapsing a bespoke static helper into a type extension (extension functions/methods, traits, mixins, or whatever the language offers) so the call site reads naturally and the helper stops being a free-floating utility.

### Don't let a file cross the size threshold without a very strong reason

Treat a PR pushing a file across the size threshold (`file_size_warn_lines` in config.json, default 1000) as a strong code-quality smell by default — the `changed_files.py` pass from setup flags these as `OVER`/`CROSSED`, so you don't have to count by hand. Prefer extracting helpers, nested types, or whole new files/modules instead of letting a file sprawl. If a file crosses that threshold, explicitly ask whether the code should be decomposed first. Only waive this if there's a compelling structural reason *and* the resulting file is still clearly organized.

Note the structural angle: many languages let you split one type across multiple files (partial types, extension files, multiple modules in one unit) — but splitting a god-class across three files doesn't fix a god-class. Prefer real decomposition (separate responsibilities into separate types/modules) over cosmetic splits that just hide the size.

### Don't allow random spaghetti growth in existing code

Be highly suspicious of new ad-hoc conditionals, scattered special cases, or one-off branches inserted into unrelated flows. If a change adds "weird `if` statements in random places," treat that as a design problem, not a stylistic nit. Prefer pushing the logic into a dedicated abstraction, helper, state machine, policy object, or strategy/handler type instead of tangling an existing path. Call out changes that make the surrounding code harder to reason about, even if they technically work.

Watch especially for: a new boolean parameter threaded through several methods to toggle behavior (usually a sign two methods are fighting to be one), repeated type-test-then-cast sequences that should be one pattern match, and null checks sprinkled to paper over an invariant that should be enforced at the boundary.

### Bias toward cleaning the design, not just accepting working code

If behavior can stay the same while the structure becomes meaningfully cleaner, push for the cleaner version. Don't rubber-stamp "it compiles and the tests pass" implementations that leave the codebase messier. Strongly prefer simplifications that remove moving pieces altogether over refactors that merely spread the same complexity around.

### Prefer direct, boring, maintainable code over hacky or magical code

Treat brittle, ad-hoc, or "magic" behavior as a code-quality problem. Be skeptical of generic mechanisms that hide simple data-shape assumptions. Concretely, be wary of:
- Reflection, runtime metaprogramming, or dynamic typing used where a normal interface, generic constraint, or pattern match would be clearer and type-safe.
- Over-clever generics/templates (deep type constraints, type gymnastics) that buy no real reuse.
- Code generation, macros, or expression/AST trickery introduced for something a plain function would handle.
- Thin abstractions, identity wrappers, or pass-through helpers that add indirection without buying clarity — a service that just forwards every call to another service, a wrapper type that adds no invariant.

### Push hard on type and nullability cleanliness when they affect maintainability

A good type system is a tool for deleting conditionals — use it that way.
- Question gratuitous nullability/optionality. A `?`/`Optional`/`null` everywhere, or a scattering of type-checker escape hatches (a force-unwrap, a non-null assertion, an `as any`), usually means an invariant isn't being enforced where it should be. Prefer making the boundary explicit (validate once, then carry a non-null / known-good type) over defensive null checks at every use.
- Be skeptical of untyped escape hatches (`object`/`any`/`dynamic`) and cast-heavy code (a hard cast, a repeated test-then-cast) where a clearer type boundary, generic, or pattern match would express the real contract.
- Prefer explicit typed models or shared contracts (named types, interfaces, structs/records, DTOs) over loosely-shaped bags — untyped maps/dictionaries, anonymous objects, or positional tuples — passed across public boundaries.
- If a branch relies on a silent fallback (a default-on-missing, an ignored lookup-failure, a swallowed error) to paper over an unclear invariant, ask whether the boundary should be made explicit instead.

### Make invalid states unrepresentable in the type/data model

Hold the data model to the same bar as the control flow: a good model makes illegal states *impossible to construct*, not merely unlikely. The failure mode to hunt is a shape that can represent combinations that should never exist — and then leans on scattered runtime checks, asserts, or comments to keep them from happening.

Watch for:
- A flag/enum plus a bag of optional fields that are only meaningful in certain combinations (a `status` field next to a `result` that's only set when `status == done` and an `error` only set when it failed). Every consumer now has to re-derive which fields are live. Model it as a closed set of cases (a sum type / discriminated union) where each case carries exactly — and only — the data valid for it.
- Primitives standing in for a constrained domain: a raw string/int where only a small set of values is legal, an unvalidated email string, a pair of numbers that must satisfy `start <= end`. Prefer a parsed/constrained type so an invalid value can't flow downstream — *parse, don't validate*: validate once at the boundary, then carry a type that proves the value is good.
- Mutable objects whose invariants can be violated between steps — a half-built object usable before it's complete, two fields that must change together but can be set independently. Prefer making invalid intermediate states unconstructible (enforce the invariant in the constructor / factory, or use immutability).

When the model lets a bad state exist, don't accept "but we check for it" — push to make the state unrepresentable so the check disappears entirely. Be as demanding here as you are about spaghetti: a PR that encodes a closed set of states as loosely-coupled optional fields (or a bare primitive), when a tighter model that forbids the bad combination is right there, is a presumptive blocker.

### Prefer a declarative style: pattern matching, immutable objects, pure functions

Where a piece of code is fundamentally a transformation or a decision over a set of cases, prefer the declarative form over imperative mechanics. This is the same "let the shape show through" instinct as the abstraction rule, applied at the small scale.

Reward — and push for when it's missing:
- **Pattern matching** over an exhaustive set of cases instead of a cascade of type-tests, casts, and `if`/`else if` on a discriminator. Exhaustive matching also turns "I forgot a case" into a detectable error rather than a silent fallthrough.
- **Immutable objects** over in-place mutation. A value that never changes after construction can't be corrupted by a later step, is safe to share, and removes a whole class of "who mutated this?" questions. Be suspicious of mutable accumulators and objects passed around to be poked at when an immutable value threaded through would say it more clearly.
- **Pure functions** over side-effecting steps. A function whose output depends only on its inputs is trivially testable and reorderable; push side effects (I/O, mutation, logging) to the edges and keep the core a pure transformation.

This is a preference, not a mandate to contort code: don't torture an inherently stateful, side-effecting, or imperative-by-nature routine into a declarative shape that reads worse. But when imperative mutation-and-branching is doing the job that a match expression over immutable data and pure transformations would do more clearly, treat the imperative version as the weaker design and say so.

### Keep logic in the canonical layer and reuse existing helpers

Call out feature logic leaking into shared paths, or implementation details leaking through public APIs. Prefer existing canonical utilities/helpers over bespoke one-offs — before accepting a new helper, ask whether the language's standard library (its collection, string, and sequence APIs), the project's existing extensions/utilities, or an existing service already does the job. Push code toward the right project, namespace, service, or layer instead of normalizing architectural drift. Domain logic belongs in the domain layer, not smeared into controllers, DTOs, or ORM/persistence entities.

### Treat unnecessary sequential orchestration and non-atomic updates as design smells

If independent async work is serialized for no good reason — a sequence of awaited calls where the operations don't depend on each other — ask whether it should run concurrently via the platform's concurrency primitive (an await-all / join / gather). If related updates can leave state half-applied (several writes without a transaction, several in-memory mutations without a clear commit point), push for a more atomic structure. Don't over-index on micro-optimizations, but do flag avoidable orchestration complexity that makes the implementation more brittle. Also flag the inverse: needless parallelism or thread/async wrapping over already-async work that just adds confusion.

## Primary Review Questions

For every meaningful change, ask:

- Is there a code-judo move that would make this dramatically simpler?
- Is there a *missing* abstraction here — a pipeline, state machine, policy, strategy set, or small algebra — that would turn shapeless procedural code into something clean? If so, what are the right 3-4 seams, and what would the call site read like afterward?
- Is this using the right paradigm for the problem's shape (functional/transformational vs. polymorphic/OO)?
- Can this be reframed so fewer concepts, branches, or helper layers are needed?
- Does this improve or worsen the local architecture?
- Did the diff add branching complexity where a better abstraction (a match expression, a sum-type model, a polymorphic dispatch) should exist?
- Does the type/data model make invalid states unrepresentable, or does it allow combinations that should be impossible — optional fields valid only in certain pairings, an enum plus side-data, a primitive where a constrained/closed set belongs?
- Could this be expressed more declaratively — pattern matching over a sum type, immutable data, pure transformations — instead of imperative mutation and ad-hoc branching?
- Did a previously cohesive type become more coupled, more stateful, or harder to scan?
- Is this logic living in the right file, namespace, and layer?
- Did this change push a file or type past a healthy size boundary?
- Are there repeated conditionals or repeated type-test-then-cast sequences that signal a missing model or missing pattern match?
- Is the implementation direct and legible, or does it lean on special cases and incidental control flow?
- Is this abstraction actually earning its keep, or is it just a wrapper / identity service?
- Did the diff introduce casts, untyped escape hatches (`object`/`any`/`dynamic`), gratuitous nullability, or type-checker silencers (a force-unwrap, a non-null assertion) to quiet the compiler instead of fixing the invariant?
- Is this logic in the canonical layer, or did the diff leak details across a boundary?
- Is this orchestration more sequential (or more parallel) than it needs to be? Could related writes be made atomic?

## What to Flag Aggressively

Escalate findings when you see:

- A complicated implementation where a cleaner reframing could delete whole categories of complexity.
- Working-but-shapeless "slop": a long procedural method or a flat step-sequence that makes no real design decisions, where the problem clearly has a shape (pipeline, state machine, producer/consumer, strategy set) that a small set of well-chosen abstractions would express cleanly. Don't pass this just because it works — name the missing abstraction.
- Refactors that move code around but fail to reduce the number of concepts a reader must hold in their head.
- A file crossing the size threshold (config.json `file_size_warn_lines`) due to the PR, especially if the new code could be split into a focused type or file.
- New conditionals bolted onto unrelated code paths.
- A data model that lets invalid states exist — fields only valid in certain combinations, a primitive standing in for a constrained set, a mutable object whose invariants can be violated between steps — where a tighter model would make the bad state impossible to construct.
- One-off boolean flags or optional "mode" fields threaded through methods to fork behavior — usually a sign two behaviors should be split into separate methods or a polymorphic dispatch.
- Imperative, mutation-heavy code that would be clearer as a declarative transformation — pattern matching instead of cascading type-tests, immutable data instead of in-place mutation, pure functions instead of side-effecting steps — without contorting genuinely stateful code.
- Feature-specific logic leaking into general-purpose modules, base classes, or shared services.
- Reflection, dynamic typing, heavy generics/templates, or code-gen/macro "magic" that hides simple structure and makes the code harder to reason about.
- Thin wrappers or identity abstractions (pass-through services, DTO-to-DTO mappers that change nothing) that add indirection without simplifying anything.
- Unnecessary casts, untyped escape hatches (`object`/`any`/`dynamic`), type-checker silencers (a force-unwrap, a non-null assertion), or gratuitous nullability/optionality that muddies the real contract.
- Copy-pasted logic instead of an extracted function/method or a type extension.
- Narrow edge-case handling jammed into the middle of an already busy method.
- Refactors that technically pass tests but make the code less modular or less readable.
- "Temporary" branching that is likely to become permanent debt.
- Bespoke helpers where the standard library or an existing canonical utility already does the job (re-implementing grouping, dedup, chunking, slicing, etc.).
- Logic added in the wrong layer/project when it should live somewhere more central.
- Sequential awaited chains over independent work where an await-all/join would be simpler and clearer; or partial-update logic that leaves state less atomic than necessary.
- Misuse of the concurrency model: fire-and-forget that drops errors, blocking on async work, wrapping already-async work in a thread, or skipping the platform's required await/context handling where it actually matters for the layer.

## Preferred Remedies

When you identify a problem, prefer suggestions like:

- Delete a whole layer of indirection rather than polishing it.
- Reframe the state model so conditionals disappear instead of getting centralized — e.g. model the cases as a sum type / closed hierarchy and match on it exhaustively.
- Tighten the data model so invalid states can't be represented at all — replace "flag + loosely-coupled optional fields" with a sum type whose cases each carry exactly their valid data; make an illegal combination a construct-time / parse-time error; *parse, don't validate* at the boundary so downstream code carries an already-valid type.
- Replace an `if`/`else if` chain or a type-test-then-cast sequence with a single match expression / pattern match on type, shape, and value.
- Push toward a declarative style — pattern-match over an exhaustive set of cases, prefer immutable values and pure functions over mutable state and side effects, and express transformations declaratively — where it reads more clearly than the imperative version.
- Change the ownership boundary so the feature becomes a natural extension of an existing abstraction (often a type extension rather than a free-floating static helper).
- Turn special-case logic into a simpler default flow with fewer exceptions.
- Extract a method, pure function, or local function.
- Split a large type into smaller focused types (real decomposition, not a cosmetic split across files).
- Move feature-specific logic behind a dedicated abstraction or into the layer that owns the concept.
- Make the type boundary explicit (non-null types, a real DTO/record, a constrained generic) so the control flow gets simpler and the nullability/casts disappear.
- Reuse the standard library or the project's canonical helper instead of a near-duplicate.
- Separate orchestration from business logic.
- Collapse duplicate branches into a single clearer flow.
- Delete wrappers that don't meaningfully clarify the API.
- Parallelize genuinely independent async work with the platform's await-all/join when that also simplifies the orchestration; or wrap related writes in a transaction / single commit point so partial state can't leak.
- Replace a manual loop + mutable accumulators with a declarative collection pipeline (map/filter/reduce) when it reads more clearly (and the reverse when a pipeline is being tortured into something a plain loop says better).

Don't be satisfied with "maybe rename this" feedback when the real issue is structural. Don't be satisfied with a merely cleaner version of the same messy idea if there's a plausible path to a much simpler idea.

## Review Tone

Be direct, serious, and demanding about quality. Don't be rude, but don't soften major maintainability issues into mild suggestions. If the code is making the codebase messier, say so clearly. If the implementation missed an opportunity for a dramatic simplification, say that clearly too.

Useful phrasings:

- "this pushes the file past the size threshold. can we decompose this type first?"
- "this adds another special-case branch into an already busy method. can we move this behind its own abstraction or a match expression?"
- "this works, but it makes the surrounding code more spaghetti. let's keep the behavior and restructure the implementation."
- "this is an enum plus three optional fields that are only valid together — can we model it as a sum type / closed case hierarchy so the invalid combination can't be represented, and match on it instead?"
- "this primitive can hold values that should be illegal here. can we parse it into a constrained type at the boundary so the bad value can't flow downstream?"
- "this feels like feature logic leaking into a shared service. can we isolate it?"
- "this abstraction seems unnecessary — it just forwards every call. can we keep the direct flow?"
- "why the type-checker silencer / the cast / the nullable here? can we enforce this invariant at the boundary so the type is non-null from then on?"
- "this looks like a bespoke helper for something the standard library / an existing extension already does. can we reuse the canonical one?"
- "these three awaits don't depend on each other — running them concurrently (await-all/join) would be simpler and faster."
- "i think there's a code-judo move here that makes this much simpler. can we reframe so these branches disappear?"
- "this works but it's shapeless — it's really a pipeline (source → dispatch → retry policy → commit). can we name those 3-4 pieces as small composable abstractions? the call site should read like that description."
- "this is a transformation, not a state machine. a declarative pass — pure steps composed over immutable data — would be far cleaner than this mutable procedural loop."
- "this refactor moves complexity around but doesn't delete it. is there a way to make the model itself simpler?"

## Output Expectations

Open the review with a single read-at-a-glance **`Verdict:`** line — `BLOCK` or `APPROVE`, defined objectively in the Approval Bar below — then the findings. Prioritize findings in this order:

1. Structural code-quality regressions
2. Missed opportunities for dramatic simplification / code-judo restructuring — *including the additive case: a missing abstraction that would turn working-but-shapeless slop into a clean small-set decomposition*
3. Spaghetti / branching complexity increases — including imperative branching/mutation where a declarative, pattern-matched form fits
4. Boundary / abstraction / type-contract problems (nullability, casts, leaky layers, invalid states left representable) that make the code harder to reason about
5. File-size and decomposition concerns
6. Modularity and abstraction issues
7. Legibility and maintainability concerns

Don't flood the review with low-value nits if there are larger structural issues. Prefer a smaller number of high-conviction comments over a long list of cosmetic notes. (Leave true cosmetic/style matters to the formatter, linter, and analyzer/style config the project already runs — unless they actively harm readability.)

## Approval Bar

Don't approve merely because behavior seems correct and it compiles. The bar for approval is:

- no clear structural regression
- no obvious missed opportunity to make the implementation dramatically simpler when such a path is visible
- no unjustified file-size explosion
- no obvious spaghetti-growth from special-case branching
- no obviously hacky or magical abstraction (reflection / dynamic typing / over-clever generics / code-gen) that makes the code harder to reason about
- no unnecessary wrapper/cast/nullability churn obscuring the real design
- no data model that leaves invalid states representable when a tighter type would forbid them
- no needlessly imperative/mutable code where a declarative, immutable, pattern-matched expression is clearly cleaner
- no clear architecture-boundary leak or avoidable canonical-helper duplication
- no missed opportunity for an obvious decomposition that would materially improve maintainability

Treat these as presumptive blockers unless the author can justify them clearly:

- the PR preserves a lot of incidental complexity when there's a plausible code-judo move that would delete it
- the PR ships working-but-shapeless code (a procedural blob, a flat step-sequence) when an elegant small-set decomposition — the right 3-4 abstractions — is clearly available
- the PR pushes a file across the size threshold (config.json `file_size_warn_lines`)
- the PR adds ad-hoc branching that makes an existing flow more tangled
- the PR encodes a closed set of states as a flag plus loosely-coupled optional fields (or a bare primitive) so invalid combinations are representable, when a sum type / tighter model would make them impossible
- the PR ships imperative, mutation-and-branching code where a declarative form — pattern matching over a sum type, immutable data, pure functions — is clearly available and reads more cleanly
- the PR solves a local problem by scattering feature checks across shared code
- the PR adds an unnecessary abstraction, wrapper, or cast/nullability-heavy contract that makes the design more indirect
- the PR duplicates an existing helper or the standard library, or puts logic in the wrong layer when there's a clear canonical home

If those conditions aren't met, leave explicit, actionable feedback and push for a cleaner decomposition.

### Emit an explicit verdict

The bar above is binary, so the review's headline is too. Open every review with one read-at-a-glance line that a human, a hook, or a CI gate can act on without parsing the prose:

- **`Verdict: BLOCK`** — at least one presumptive blocker above is present and not clearly justified by the author, **or** the `changed_files.py` pass reported an `OVER`/`CROSSED` file that wasn't waived with a stated structural reason. Name every blocker that fired: `Verdict: BLOCK — <blocker>; <blocker>`.
- **`Verdict: APPROVE`** — every condition above holds, and any file-size flag was explicitly waived with a reason.

The verdict is a direct function of the bar, not a vibe — if you catch yourself wanting to "approve with reservations," that is a `BLOCK` with the reservation named. Optionally add a secondary `Structure: <n>/10` maintainability grade for trend-tracking, but the `BLOCK`/`APPROVE` gate is the verdict that counts.
