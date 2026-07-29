You own issue {{ISSUE_ID}} end-to-end in your own worktree at `{{WORKTREE_PATH}}`. There
is no shared merge — you take this issue alone to its PR. Read its context from
`{{CONTEXT_FILE}}`.

**0. Orient.** Read the target repo's `CLAUDE.md` / `CONTEXT.md` and the ADRs near the
code you'll touch. Match its conventions and domain vocabulary.

**1. Implement.** Use the `tdd` skill (this repo's `tdd`, NOT
`superpowers:test-driven-development`) together with `ponytail`: red → green → refactor,
one vertical tracer-bullet test at a time, always the laziest working solution. Mark any
deliberate simplification with a `ponytail:` comment. (No ponytail plugin? Skip the markers.)

**2. Review loop (≤ {{MAX_REVIEW_PASSES}} passes).** Repeat over your own diff: run
`thermo-nuclear-code-quality-review`, then fix every finding that is a **real regression**
(spaghetti, correctness, maintainability). Precedence on conflict: `ponytail` wins; a
pure "more abstraction" finding against `ponytail:`-marked code counts as accepted, not a
regression. Stop as soon as no real-regression findings remain, or after
{{MAX_REVIEW_PASSES}} passes — report any remaining findings in the PR body.

**3. Finish.** Commit everything; your worktree's working tree MUST be clean. Open exactly
one PR for issue {{ISSUE_ID}}, linking the issue, with leftover review findings noted in
the body.
