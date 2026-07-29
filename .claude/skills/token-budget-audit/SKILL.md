---
name: token-budget-audit
description: Audit a repo's persistent-context artifacts (CLAUDE.md, SKILL.md files, memory files) against token-budget heuristics from cost-optimization practice — oversized instruction files, context duplicated across multiple files instead of centralized, and missing ignore-file coverage for large irrelevant paths — and return a PASS/FAIL verdict plus a located, itemized report. Use when the user wants to "audit token budget", "check CLAUDE.md size", "Kontext-Kosten im Repo prüfen", "is our CLAUDE.md too bloated", "token cost audit for skills".
arguments: Optional. Scope = repo root or a subdirectory to audit (default — the whole repo).
---

# Token Budget Audit

Check whether this repo's always-loaded context — `CLAUDE.md` files, `SKILL.md` files, memory files — is sized and organized so it doesn't quietly inflate every prompt's token cost. This is a **checker, not a fixer**: it locates and sizes the bloat, it does not trim or rewrite anything (that's `compress-prompt`'s job, run separately per file).

## Verdict

- **PASS** — no `high` finding, and at most incidental `medium`/`low` findings.
- **FAIL** — any `high` finding.

**Severity rubric:**

- **high** — a file that's always loaded into context (root `CLAUDE.md`, a frequently-triggered `SKILL.md`) is grossly oversized for what it needs to convey, or the same non-trivial block of instructions is duplicated verbatim across multiple always-loaded files.
- **medium** — a single `SKILL.md` is noticeably larger than its peers without a clear reason (bundled reference tables, large embedded templates), or a large generated/vendor directory has no ignore-file entry and would be swept into context on a broad read.
- **low** — verbose but harmless prose (filler, redundant explanation) in an occasionally-loaded file.

## Checks

The mechanical checks below (1, 2, 4) are scripted; the judgement checks (3, 5) are yours — same split as `docs-code-consistency`'s scan/probe.

1. **CLAUDE.md size.** Every `CLAUDE.md` in scope against `claude_md_token_guideline` in the bundled `config.json` (ships as 1,500 — the rough guideline from cost-optimization practice for what an always-loaded root file needs).
2. **SKILL.md size outliers.** Each `SKILL.md`'s size against the average of its siblings (other `SKILL.md` files under the same parent directory), flagged past `skill_size_outlier_multiplier` (ships as 4x).
3. **Duplicated context** *(judgement)*. Look for the same non-trivial instruction block appearing in more than one always-loaded file (root `CLAUDE.md` vs. a nested one, or repeated across several `SKILL.md` files) instead of living once in a shared reference.
4. **Ignore-file coverage.** Any directory named in `large_dir_names` (config.json — `node_modules`, build output, etc.) that exists under scope but isn't covered by a `.gitignore`/`.claudeignore` between it and the repo root.
5. **Verbose prose** *(judgement)*. For files already flagged above, note (don't fix) where the same politeness/filler/redundancy patterns `compress-prompt` targets are present — cross-reference that skill for the fix pass instead of repeating its technique list here.

## Process

1. **Resolve scope** from `arguments` (default: whole repo).
2. **Run the mechanical pass** — don't re-derive file discovery, sizing, or ignore-coverage by hand:

   ```bash
   uv run .claude/skills/token-budget-audit/scripts/token_budget.py <scope-path>
   ```

   Prints `files`, `claude_md_flags`, `skill_size_outliers`, and `uncovered_large_dirs` as JSON.
3. **Judgement pass.** Read the flagged files (and any memory files referenced in the session) for duplicated instruction blocks and verbose prose per checks 3 and 5.
4. **Assign severity** per finding using the rubric, then combine into PASS/FAIL.

## Report format

Read the bundled template and fill it — don't re-derive the layout inline:

```
<this-skill's-base-dir>/assets/report-template.md
```

One row per finding in the table (highest severity first); skip the row placeholder entirely if there are none. `{{TOP_FIX}}` is the single highest-value fix, or omit the sentence if the verdict is a clean PASS.
