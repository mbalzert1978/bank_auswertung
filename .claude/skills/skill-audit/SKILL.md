---
name: skill-audit
description: Audit the user's existing skills and flag any that straddle more than one bucket, recommending a concrete split or trim so each fits one bucket cleanly. Use when the user asks to review or audit their existing skills, clean up skills that try to do too much, or check whether a skill straddles buckets. To find NEW skills they're missing from their chat history, use propose-skills instead.
arguments: Optional. The skill source to audit — a path (e.g. `.claude/skills/`) or a scope. If omitted, the skill asks which source to audit.
---

# Skill Audit

Look at the user's existing skills and flag any that try to live in more than one bucket, then say concretely how to split or trim each so it fits one bucket cleanly.

This skill only **flags straddlers among existing skills**. To find *new* skills the user is missing from their chat history, use `propose-skills`.

Every existing skill gets sorted into exactly one of four buckets — the ones that won't sort cleanly are the straddlers.

## The four buckets

1. **Utility** — does one small, reusable thing, the same way every time (format, convert, a single lookup).
2. **Verification** — checks the quality of a final output (lint, validate, review against a rubric).
3. **Data enrichment** — pulls external data *in* (fetches, queries, scrapes context the agent doesn't already have).
4. **Orchestration** — chains other skills into a multi-step playbook.

## The rule

A good skill fits **cleanly into one bucket**. A skill that straddles two or more buckets confuses the agent about when to fire it.

**Exception:** an orchestration skill that *coordinates* other skills is not straddling — coordinating is its one job. Don't flag orchestration for "doing several things"; that's the point of the bucket.

## Process

### 1. Resolve scope

Figure out *which* skills to audit before doing anything else.

- **If a target was passed** (a path or scope in the invocation arguments), use it as the source and skip straight to inventory.
- **If no argument was given**, call `AskUserQuestion` to let the user pick — don't guess and don't ask in free-form prose. Use one question, header `Audit scope`, with these options:
  - **Project skills** — `.claude/skills/` in the current project.
  - **Personal skills** — `.claude/skills/` (the user's central skill store).
  - **All sources** — project + personal + any skills referenced in memory.

  `AskUserQuestion` adds an **Other** choice automatically, so the user can type a custom path or source without it being listed here.

Whatever scope is chosen governs steps 2 and 3 (which skills get inventoried and checked for straddling).

### 2. Inventory existing skills

Run this skill's bundled inventory script over the chosen scope — don't re-derive skill discovery by hand:

```bash
uv run .claude/skills/skill-audit/scripts/inventory.py <scope-path>
```

It prints a JSON array, one object per skill (`name`, `description`, `arguments`, `path`, `has_scripts`, `has_assets`, `has_config`). Add any skills referenced only in memory that the script can't see. Then, for each skill, note the one job it does and which bucket that job belongs to.

### 3. Flag straddlers

Go through the existing skills and flag any that span 2+ buckets. For each, say concretely how to fix it:

- **Split** — break it into two skills, one per bucket, and name each.
- **Trim** — cut the scope down to the single bucket it mostly belongs to, and name what to remove.

## Report format

Be blunt. Focus on real straddlers — **skip skills that are already clean.** Don't pad the report with skills that are fine.

**Straddlers to fix**

| Skill | Buckets it spans | Fix (split / trim) |
| ----- | ---------------- | ------------------ |

End with the single highest-value action: the one straddler worth fixing first, and why. If nothing straddles, say so plainly.
