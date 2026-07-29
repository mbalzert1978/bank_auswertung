---
name: docs-code-consistency
description: "Check that a repo's documentation (docstrings, README, prose docs/ADRs/CONTEXT.md) is consistent with the current code and return an objective PASS/FAIL verdict plus an itemized, located drift report. Use when the user wants to check docs against the code — \"check the docstrings still match the code\", \"schau nach ob alle docstrings dem code entsprechen\", \"is the README still accurate\", \"Doku-/Artefakt-Konsistenz zum Code prüfen\", \"verify the docs match the code\", \"README drifted from the real workflow\"."
arguments: "Optional. Scope = the package/dir to check (default: the whole repo). Optionally the doc types to check — docstrings / README / docs+ADRs+CONTEXT.md (default: all). Optionally a severity threshold for the pass criterion — `high` or `medium` (default: config.json `severity_default`, ships as `medium`)."
---

# Docs ↔ Code Consistency

Check that the repository's **documentation** still matches the **current code**, and
return an objective verdict: **PASS / FAIL**, plus an itemized drift report. The output
under test is the docs; the code is ground truth.

This is a **checker, not a fixer**. Detecting and reporting drift — grounded and located —
is the whole job. It does **not** edit docs or code (see *Out of scope* below).

## Verdict

The pass criterion is explicit and testable — no "looks mostly fine":

- **PASS** — **zero** drift items at or above the agreed severity threshold (default is
  `severity_default` in the bundled `config.json`, ships as **medium**: PASS requires zero
  `high` and zero `medium` items; `low`/cosmetic items are reported but do not affect the
  verdict). State the count you passed against.
- **FAIL** — **any** drift item at or above the threshold. Name the count and the worst item.

Severity threshold is set by the `arguments` (`high` = fail only on high; `medium` = fail on
high or medium). Optionally also emit a **grade /10** (10 = no drift of any severity; subtract
for each item weighted by severity) — but the PASS/FAIL gate is the verdict that matters.

**Severity rubric** (assign one to every item):

- **high** — the docs *contradict* the code: a documented param/return/raised-exception/flag
  that's wrong, a README/usage command that fails or references a removed entry point, a
  documented symbol/path/flag that no longer exists. A reader following the docs breaks.
- **medium** — misleading but not immediately breaking: a stale code example, an outdated
  workflow that still half-works, a public surface the docs claim to cover but don't.
- **low** — cosmetic: a still-resolving typo, slightly dated phrasing.

## The iron rule — grounded findings, no false positives

A drift item is worthless unless it's **grounded and located**. Every item must carry
`{doc location → code location, the exact mismatch, severity}`.

- **Verify before flagging.** Before recording "documented X no longer exists", confirm X is
  actually absent with `scripts/doc_consistency.py probe` (see below) — and the reverse for
  "code surface Y is undocumented". A false positive is a failure of *this* skill.
- **No speculation.** If you can't point to both ends (the doc line and the code reality),
  don't record it. The script's `scan` output is a *candidate* list — an unresolved reference
  may be an intentional illustrative/negative example; confirm each before it becomes drift.
- **Preserve language.** Write each finding in the repo/user's language (German or English) —
  quote the doc verbatim.

## Scope of drift to check

1. **Docstrings ↔ reality** — documented params, return types, raised exceptions, and
   described behaviour match the actual signature and implementation. — *judgement*
2. **README ↔ reality** — documented install/usage/workflow commands and entry points exist
   and reflect the current setup (e.g. a README still describing the old workflow after a move
   to `uv`). — *mechanical (file targets) + judgement (does the command still do what it says)*
3. **Doc artifacts (`docs/`, ADRs, `CONTEXT.md`) ↔ code** — referenced files/functions/flags
   still exist; code examples match current signatures; links and anchors resolve. — *mechanical
   (refs/links) + judgement (examples)*
4. **Stale references, both directions** — documented symbols/paths/flags that no longer exist
   (script-confirmed absent), **and** public code surfaces the docs claim to cover but don't. — *both*

## Mechanical vs. judgement split

The deterministic existence/resolution checks are scripted; the prose-matches-behaviour checks
are yours. Run the script first, then judge.

```bash
# 1. Unresolved references in markdown docs (broken links, dead anchors, dead file paths,
#    dead fenced-command targets). Discovery defaults (doc_globs / doc_dirs) live in the
#    bundled config.json — README*, docs/, CONTEXT*, CHANGELOG* by default. Pass explicit
#    DOC_PATHs to narrow, or set doc_dirs in config.json for repos that keep docs elsewhere.
uv run .claude/skills/docs-code-consistency/scripts/doc_consistency.py scan --repo <repo> [DOC_PATH ...] --json

# 2. Confirm a token (symbol / flag / path) really is present/absent in the CODE before you
#    flag drift in either direction. Pass the candidates you found while reading the docs.
uv run .claude/skills/docs-code-consistency/scripts/doc_consistency.py probe <repo> --repo <repo> \
    parse_config --max-retries src/old_module.py
```

`scan` is high-precision by design (it under-extracts rather than emit false positives).
`probe` is the existence proof the iron rule demands — use it for every "X no longer exists"
claim. Docstring param/return/behaviour matching is **not** scripted (it spans Python, Rust,
etc.); read the source and judge, using `probe` to confirm any symbol you cite.

## Lookup

Use the project's code search to locate doc↔code links: prefer the user's **`semble`** MCP
(`mcp__semble__search`) / CLI for semantic lookup ("where is the behaviour this docstring
describes?"); fall back to **grep/glob** (and `probe`) for exact symbol/path/flag existence.
Read `CONTEXT.md` / `docs/adr/` the way **`grill-with-docs`** and **`deepen-module`** maintain
them — check the *same* artifacts, don't invent a parallel set.

## Process

1. **Resolve scope.** Take the package/dir, doc types, and severity threshold from `arguments`.
   If scope/doc-types are missing **and** the repo is large, ask once with `AskUserQuestion`
   (header `Scope`) — which dir, and which doc types (docstrings / README / docs+ADRs+CONTEXT.md
   / all). Otherwise default to the whole repo, all doc types, threshold `medium`.
2. **Mechanical pass.** Run `scan` over the doc set. For each unresolved reference, confirm it's
   genuine drift (not an illustrative example) — discard the rest.
3. **Judgement pass.** For each doc claim about behaviour (docstrings, README workflow, code
   examples in ADRs/docs), find the code (semble → grep) and compare. Record only mismatches you
   can locate at both ends. Use `probe` to prove existence/absence for every symbol/flag/path
   you cite, in both directions (stale doc ref *and* uncovered public surface).
4. **Assign severity** to each item via the rubric.
5. **Combine.** PASS iff no item is at or above the threshold; else FAIL. Order items by severity.

## Out of scope — this is a checker

The user's "Cleanup / Konsistenz herstellen" wording mixes in *fixing*. Fixing the drift is a
**separate concern** and is **not** done here. Flag this explicitly in the report and make every
item copy-paste-actionable (exact location + exact change implied) so a follow-up fix pass can
apply it cleanly. Do **not** add a fix mode to this skill.

## Output format

Lead with the verdict, then the itemized drift table (highest severity first), then the scope line
and the fix-is-separate note. Don't pad with what was fine.

**Verdict: PASS** — *e.g.* "zero high/medium drift across 23 docstrings, README, and 6 doc
artifacts (2 low items noted)." *(threshold: medium)* — *Grade: 9/10 (optional)*

or

**Verdict: FAIL** — *e.g.* "5 drift items at or above medium (2 high)." *(threshold: medium)*

| Sev | Doc location | Code location | Mismatch | Fix implied |
| --- | ------------ | ------------- | -------- | ----------- |
| high | `README.md:14` | `pyproject.toml` / `uv.lock` | README says `pip install -r requirements.txt`; project uses the `uv` workflow (no requirements.txt) | rewrite the install section to `uv sync` |
| high | `src/cfg.py:30` docstring | `src/cfg.py:34` signature | docstring documents param `retries`; the parameter is `max_retries` | rename in docstring |
| medium | `docs/adr/0002.md:21` | `src/store.py` | ADR example calls `Store.save(obj, flush=True)`; `save()` no longer takes `flush` | update the example |

- **Scope checked:** *what doc types / dir, and the threshold used.*
- **Fix is out of scope:** this run only reports drift; apply the *Fix implied* column in a
  separate pass (a fixer skill or a manual edit). End with the single highest-value item to fix first.
