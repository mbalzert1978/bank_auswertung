---
name: verifier-audit
description: Audit the user's existing skills to find the ones that should become verifiers or gain a verification step — skills that produce output but never check it, and skills whose verdict is vague instead of an objective Pass/Fail or grade out of 10. Use when the user wants to turn skills into verifiers, add objective pass/fail or scoring checks, make a subjective verdict objective, or find which skills should verify their own output.
arguments: Optional. The skill source to audit — a path (e.g. `.claude/skills/`) or a scope. If omitted, the skill asks which source to audit.
---

# Verifier Audit

Go through the user's existing skills and find the ones that should be turned into verifiers — or have a verification component bolted on. This audits whether a skill *checks its own output*, not whether it earns its place (`skill-audit`) or how it's mechanically built (`skill-tune-up`).

## What makes a good verifier

A good verifier has an **objective output**: a clear **Pass/Fail**, or a **grade out of 10**. A vague, subjective verdict ("looks good", "seems solid") is not a verifier.

It checks one of two things:

1. **Correctness** — are the facts, numbers, and quoted sources real and right?
2. **Quality** — does the output actually meet the bar the user wants?

## The three findings

For each skill, look for one of these:

1. **Producer with no check.** The skill PRODUCES output but never CHECKS it — a writer, generator, or drafter. Surface what a Pass/Fail or /10 check *on its output* would look like.
2. **Subjective verifier.** The skill already verifies something but gives a vague, subjective verdict. Say how to make the verdict objective (Pass/Fail or /10).
3. **Borrow, don't build.** A verifier need is better served by an existing skill the producer can CALL than by a new check — e.g. a brand-voice skill a `report-reviewer` calls to pass/fail tone. Recommend the borrow.

## Process

### 1. Resolve scope

Figure out *which* skills to audit before doing anything else.

- **If a target was passed** (a path or scope in the invocation arguments), use it as the source and skip straight to inventory.
- **If no argument was given**, call `AskUserQuestion` to let the user pick — don't guess and don't ask in free-form prose. Use one question, header `Audit scope`, with these options:
  - **Project skills** — `.claude/skills/` in the current project.
  - **Personal skills** — `.claude/skills/` (the user's central skill store).
  - **All sources** — project + personal + any skills referenced in memory.

  `AskUserQuestion` adds an **Other** choice automatically, so the user can type a custom path or source without it being listed here.

### 2. Inventory

Run this skill's bundled inventory script over the chosen scope — don't re-derive skill discovery by hand:

```bash
uv run .claude/skills/verifier-audit/scripts/inventory.py <scope-path>
```

It prints a JSON array, one object per skill (`name`, `description`, `arguments`, `path`, `has_scripts`, `has_assets`, `has_config`). Add any skills referenced only in memory that the script can't see.

### 3. Audit each skill

Read the actual SKILL.md — don't guess from the name. For each skill, decide whether it produces output, whether it already checks anything, and whether that check is objective. Then place it under one of the three findings, or skip it. A skill can have zero findings.

For every finding, write the concrete verifier: the exact Pass/Fail condition or the /10 rubric, and whether it checks **correctness** or **quality**.

### 4. Report

Rank by impact: which **2–3 tweaks** would raise output quality the most. Be blunt; skip skills that don't apply — don't pad the report.

## Report format

Only include a section if it has rows.

**Producers missing a verifier**

| Skill | What it produces | Check on its output (Pass/Fail or /10) | Correctness or Quality |
| ----- | ---------------- | -------------------------------------- | ---------------------- |

**Subjective verdicts to harden**

| Skill | Current vague verdict | Objective replacement (Pass/Fail or /10) |
| ----- | --------------------- | ---------------------------------------- |

**Borrow, don't build**

| Verifier need | Existing skill to borrow | How it plugs in |
| ------------- | ------------------------ | --------------- |

End with **the top 2–3 tweaks by impact** — the changes that would raise output quality the most, highest first, each with a one-line why.
