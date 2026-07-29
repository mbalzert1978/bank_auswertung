---
name: propose-skills
description: Scan the user's past chat history for repeated tasks they do by hand that should become skills, classify each candidate into exactly one of the four buckets, and propose it (name, bucket, the one job it does) — skipping anything an existing skill already covers. Use when the user asks what skills they're missing, to find skill candidates or gaps from their chat history, to propose new skills to add, or "what should I turn into a skill". This finds NEW skills to add only; flagging existing skills that straddle buckets is skill-audit's job.
arguments: Optional. The existing-skill source to dedupe candidates against — a path (e.g. `.claude/skills/`) or a scope. If omitted, the skill asks which source.
---

# Propose Skills

Scan the user's past chat history for repeated tasks they do by hand that should be skills. For each real candidate, propose a name, the single bucket it belongs to, and the one job it does.

This skill only **finds new skills to add**. Flagging *existing* skills that straddle buckets is `skill-audit`'s job — don't do that here.

## The four buckets

Every candidate must fit **cleanly into exactly one**:

1. **Utility** — one small, reusable thing, the same way every time (format, convert, a single lookup).
2. **Verification** — checks the quality of a final output (lint, validate, review against a rubric).
3. **Data enrichment** — pulls external data *in* (fetches, queries, scrapes context the agent doesn't already have).
4. **Orchestration** — chains other skills into a multi-step playbook.

## The rule

A good skill fits cleanly into one bucket. If a candidate seems to need two, scope it down to its core job — or it's actually two separate candidates; propose both.

**Exception:** an orchestration candidate that *coordinates* other skills is not straddling — coordinating many steps is its one job. Don't over-split an orchestration candidate to make it look single-purpose.

## Process

### 1. Resolve which skills already exist

You need to know what's already covered so you don't propose a duplicate.

- **If a target was passed** (a path or scope in the invocation arguments), use it as the source and skip to inventory.
- **If no argument was given**, call `AskUserQuestion` — don't guess, don't ask in free-form prose. One question, header `Skill source`, options:
  - **Project skills** — `.claude/skills/` in the current project.
  - **Personal skills** — `.claude/skills/` (the user's central skill store).
  - **All sources** — project + personal + any skills referenced in memory.

  `AskUserQuestion` adds an **Other** choice automatically, so the user can type a custom path.

### 2. Inventory existing skills

Run this skill's bundled inventory script over the chosen scope — don't re-derive skill discovery by hand:

```bash
uv run .claude/skills/propose-skills/scripts/inventory.py <scope-path>
```

It prints a JSON array, one object per skill (`name`, `description`, `arguments`, `path`, `has_scripts`, `has_assets`, `has_config`). Add any skills referenced only in memory that the script can't see. For each, note the one job it does — that's your dedupe list.

### 3. Scan chat history for repeated manual tasks

Look through past conversations for tasks the user repeats by hand. A task qualifies as a candidate only if **all three** hold:

- It **recurs** — done more than once, or clearly a standing habit (not a one-off).
- It's **mechanical and repeatable** enough to capture as a fixed procedure.
- **No existing skill already covers it** (check against the step 2 list).

Skip one-offs, throwaway exploration, and anything an existing skill handles.

### 4. Classify each candidate into one bucket

For every surviving candidate, assign exactly one bucket. If it seems to span two, trim it to its core job — or split it into two candidates and list both. Apply the orchestration exception.

For each candidate, settle on:

- **Name** — kebab-case, names the one job.
- **Bucket** — exactly one of the four.
- **One job** — a single sentence describing what it does.

## Report format

Be blunt. Surface only real, repeated candidates — **skip maybes and one-offs.** Don't pad the report.

**Skill candidates to add**

| Name | Bucket | One job |
| ---- | ------ | ------- |
| `kebab-name` | Utility / Verification / Data enrichment / Orchestration | one sentence |

End with the single highest-value skill to add first, and why.
