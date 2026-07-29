---
name: skill-tune-up
description: Audit every skill in the project for mechanical weaknesses — deterministic logic that should be a bundled script, templated output that belongs in assets/, repeated config that belongs in config.json, multiple-choice setup that should use AskUserQuestion, and invocation-time inputs that should be an `arguments` frontmatter field. Use when the user wants to harden, tighten, refactor, or improve the structure of their existing skills.
arguments: Optional. The skill source to audit — a path (e.g. `.claude/skills/`) or a scope. If omitted, the skill asks which source to audit.
---

# Skill Tune-Up

Go through **every skill in the chosen scope** (see step 1) and, for each one, flag what's missing or weak against the five structural levers below — then suggest a concrete improvement and the change it would take.

This audits the *mechanics* of how a skill is built, not whether it earns its place. (For bucket-fit and gaps, see the `skill-audit` skill.)

## The five checks

For each skill, ask:

1. **Deterministic logic → script.** Is there a step that always runs the same way (parsing, formatting, computing, file shuffling) that the SKILL.md currently spells out in prose? Move it into a bundled script the skill calls, so the agent runs code instead of re-deriving the steps each time — more reliable, fewer tokens.

2. **Templated output → `assets/`.** Is there boilerplate the skill reproduces — a report scaffold, an HTML shell, a standard file header? Move it into `assets/` and have the skill read and fill the template, instead of regenerating the boilerplate inline every run.

3. **Re-entered config → `config.json`.** Is there a value the user keeps supplying, or one hard-coded in the prose (paths, model ids, thresholds, repo names)? Lift it into `config.json` so it's set once and read, not re-entered.

4. **Multiple-choice setup → `AskUserQuestion`.** Does the skill open with free-form questions whose answers are really a small fixed set of options? Replace that with `AskUserQuestion` so setup is structured instead of a back-and-forth.

5. **Invocation-time inputs → `arguments` frontmatter.** Does the skill need a slug, file path, or target handed in when it's invoked? Declare an `arguments` frontmatter field so those inputs are passed at call time rather than fished out of the conversation.

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

Run this skill's bundled inventory script over the chosen scope — don't re-derive skill discovery or bundled-file detection by hand:

```bash
uv run .claude/skills/skill-tune-up/scripts/inventory.py <scope-path>
```

It prints a JSON array, one object per skill, including `has_scripts`, `has_assets`, and `has_config` — so you don't recommend what already exists. Add any skills referenced only in memory that the script can't see.

### 3. Audit each skill

Run the five checks against each skill. Read the actual SKILL.md — don't guess from the name. Note only the checks that genuinely apply; a skill can have zero findings.

### 4. Report

For each skill **with findings**, state what's weak and the concrete change needed. Skip skills that are already clean — don't pad the report.

## Report format

Per skill, only if it has findings:

**`<skill-name>`**
- **<lever>** — what's weak → the change to make.

End with the single **highest-leverage change** across all skills: the one fix worth doing first, and why.
