---
name: build-verifier
description: Scaffold a new verification skill from a description of what to verify — pins down the objective verdict (Pass/Fail or a grade out of 10) and its testable criteria, identifies the external data and tool the check needs, reuses existing skills before building, and drafts the SKILL.md. Use when the user wants to build a verifier or a verification skill from scratch, create a Pass/Fail or scoring check for some output, or says "build a verifier for X".
arguments: Optional. What to verify — a description of the output or artifact the new skill should check (the `[WHAT I WANT TO VERIFY]` target). If omitted, the skill asks.
---

# Build a Verifier

Help the user build a verification skill from scratch for a thing they name. The output of *this* skill is a drafted `SKILL.md`; the skill it produces sits in the **Verification** bucket.

## What makes a good verifier

A good verifier has an **objective output** — **Pass/Fail**, or a **grade out of 10** — that either the user or Claude can read at a glance. It checks **correctness** (facts, numbers, sources real and right) and/or **quality** (meets the bar). If it can't emit an objective verdict, it isn't a verifier yet.

## Process

### 1. Resolve the target

What is being verified?

- **If a target was passed** in the invocation arguments, use it.
- **If not**, ask the user in one short question what output or artifact the verifier should check. This is open-ended, so plain prose is fine here — don't force `AskUserQuestion`.

Restate the target in one sentence before going on.

### 2. Define the objective verdict

Decide the shape of the verdict with `AskUserQuestion` — don't guess. One question, header `Verdict`, options:

- **Pass / Fail** — a binary gate.
- **Grade out of 10** — a graded score.

`AskUserQuestion` adds an **Other** choice automatically.

Then list the **exact criteria** the verifier checks. Each criterion must be **testable** — a yes/no or a measurable threshold, not a vibe. For each, mark whether it checks **correctness** or **quality**.

### 3. Identify external data and tools

Work out what **external data** the check needs to do its job (facts to confirm, a source to fetch, analytics, a file, a transcript) and **which tool pulls it in** (web search, a specific API, file read, an MCP server). **Flag anything the user doesn't already have wired up** so it's a known prerequisite, not a surprise at run time.

### 4. Reuse before building

Before drafting, check whether an existing skill already covers part of this. Run the bundled inventory script over the user's skills — don't re-derive discovery by hand:

```bash
uv run .claude/skills/build-verifier/scripts/inventory.py .claude/skills/
uv run .claude/skills/build-verifier/scripts/inventory.py .claude/skills/
```

It prints a JSON array, one object per skill (`name`, `description`, `arguments`, `path`, `has_scripts`, `has_assets`, `has_config`). If a skill already does the check or pulls the data, recommend the verifier **call or borrow it** rather than duplicating it.

### 5. Draft the skill

Fill the bundled template — read it, don't regenerate the scaffold inline:

```
<this-skill's-base-dir>/assets/verifier-skill.template.md
```

Fill in: **name** (kebab-case, names the one job), **bucket** (Verification), **inputs** (`arguments`), the **check steps**, and the **exact format** of the Pass/Fail or grade output.

## Keep it to one job

A verifier checks **one** thing. If the target is trying to verify more than one thing, **split it** into separate verifier skills and name each — don't ship a verifier that straddles. Surface the split before drafting.

## Report format

Lead with the drafted `SKILL.md` (the filled template), then a short summary:

- **Verdict** — Pass/Fail or /10, and the criteria, each tagged correctness or quality.
- **External data** — what it needs, which tool, and anything **not yet wired up**.
- **Reuse** — existing skill to borrow, or "none — build new".
- **Split** — only if the target had to be broken into more than one verifier.
