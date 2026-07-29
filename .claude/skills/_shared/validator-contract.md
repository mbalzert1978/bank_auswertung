# Validator Contract (shared)

Every validator skill dispatched by `validate-fix-loop` (and usable standalone) follows
this contract. Referenced by name from each validator's own `SKILL.md` instead of
restated — the prose equivalent of the `_shared/*.py` symlink convention documented in
`.claude/skills/CLAUDE.md`: one source of truth instead of N drifting copies. Edit this
file, not a per-skill restatement of it.

## Repo- and language-neutrality

A validator states its rule structurally and never anchors it in one repo's files,
documents, or language. No citing a specific `ADR-####`, `CLAUDE.md` passage or rules
directory as the reason a rule exists; no assuming a language's keywords, exception
types or visibility mechanics. Where a genuine exception to a smell exists — a
deliberate architectural seam, an intentionally thin boundary type — state it as the
principle it is ("an intentional port/adapter boundary is not a finding"), so it holds
in any repo, and identify the language from the files under review before applying any
language-shaped judgment.

## Scope resolution (diff-scoped validators)

Resolve the target from `arguments`: a file/dir path, a PR number, or a base branch /
diff range. If nothing was passed, review the current branch's changes against its
merge-base with the default branch.

(Skills with a different scope shape — e.g. `qa-check`'s test-suite run,
`lint-and-format-check`'s fixed config-driven command — document their own scope
instead of pointing here.)

## `Findings: <n>` trailer

End every non-abort report with a line `Findings: <n>`, where `<n>` is the count of
concrete, located findings the report actually lists (`0` for a clean pass). This is
what lets `validate-fix-loop` — or any other caller — parse the result mechanically
without re-reading prose. Each skill defines locally *what* counts as a finding; only
the trailer format itself is shared.

## `Verdict: CONFIG ERROR` abort

If a skill's required `config.json` key(s) are missing, stop before dispatching any
further work and report `Verdict: CONFIG ERROR` plus which key(s) are missing and
where to set them. Do **not** follow it with a `Findings:` line — a config error is
not a finding count. `validate-fix-loop` checks for the `CONFIG ERROR` marker before
it ever looks for `Findings:`, and treats it as an immediate stop (no fixer dispatch,
no further iterations) — not something a fixer can act on.
