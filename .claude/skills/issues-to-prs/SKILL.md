---
name: issues-to-prs
description: Implement each selected open issue as an independent, parallel git-worktree agent and ship exactly one PR per issue — no shared merge step. Selects the issues, gates them through a disjointness check, fans the non-overlapping ones out to one Opus agent per issue (each in its own worktree), and serializes overlapping ones in a single strand. Each agent implements with tdd + ponytail, runs a bounded thermo-nuclear review loop, and opens one PR linking its issue. Use when the user wants to "implement the open issues, one agent one issue one PR", "offene Issues implementieren — ein Agent, ein Issue, ein PR", "Implementiere die Issues mit maximaler Parallelität", or "ship a PR per open issue".
arguments: Optional. Which issues to work — `all` open (default), a label (e.g. `label:ready`), or explicit numbers (e.g. `#41 #42 #57`) — plus optional scope. If omitted, default to all open issues and confirm first, or ask for the selection mode.
---

# Issues to PRs

Take the open issues and ship **one PR per issue**, each built independently and in
parallel. This is the consumer end of the issue pipeline (`audit-to-issues` →
`to-issues` → **issues-to-prs**): it only *consumes* existing issues and produces PRs.
Creating, auditing, and re-verifying the breakdown belong to `to-issues`,
`audit-to-issues`, and `verify-issue-breakdown` — not here.

The skill's one job is **coordination**. Everything substantive is delegated:
implementation to the `tdd` skill + the `ponytail` plugin, per-issue review to
`thermo-nuclear-code-quality-review`. Parallel dispatch and worktree isolation are
done here with built-in tools — the Agent/Task tool and plain `git worktree`. **Do
not pull in any superpowers / superpowers-dev skill, and never use
`superpowers:test-driven-development` — use this repo's `tdd`.**

## Iron guards — the contract (check every one)

- [ ] **Disjointness gate runs BEFORE dispatch.** Only non-overlapping issues run in
      parallel. Two issues that would touch the same code region are serialized in one
      strand — never parallelized. When unsure whether two issues overlap, treat them as
      overlapping (serialize). A wrong "disjoint" call causes merge conflicts; that is the
      expensive failure, so bias to serial.
- [ ] **One issue → one agent's PR.** Each issue ships exactly one PR linking that issue.
      Never bundle two issues into one branch or one PR.
- [ ] **No shared merge step.** Every PR stands alone. There is no integration branch that
      collects the strands.
- [ ] **Clean worktree before the PR.** An issue's PR is opened only after its worktree's
      working tree is clean (everything committed).
- [ ] **Bounded review loop.** ≤ `max_review_passes` thermo-nuclear passes per issue (from
      `config.json`, default 3). `ponytail` wins ties; thermo-nuclear overrides only for real
      regressions. Leftover findings go in the PR body.
- [ ] **Opus + maximum reasoning effort** for every dispatched agent (set as dispatch params).
- [ ] **Honor the target repo's own quality bar.** Each agent reads the target repo's
      `CLAUDE.md` / `CONTEXT.md` and follows its conventions and ADRs. Keep this general —
      do not assume any one project's rules.

## Delegates & graceful degradation

| Step | Delegate to | If unavailable |
|------|-------------|----------------|
| Implementation | this repo's `tdd` skill | `tdd` is a repo skill — always present here |
| Deliberate-simplification discipline | `ponytail` plugin (`ponytail:` markers, tie-break precedence) | skip the markers and the ponytail tie-break; still apply `tdd` + thermo-nuclear and fix only real regressions |
| Per-issue review | `thermo-nuclear-code-quality-review` | repo skill — present here; on non-C#/.NET stacks apply its structure-first lens against the repo's own bar |
| Parallel dispatch | built-in Agent/Task tool (one agent per strand) | if parallel dispatch isn't available, run the strands sequentially — the per-issue PRs are unchanged |
| Worktree isolation | `git worktree add` (built-in) | if worktrees can't be created, run strands one at a time on sequential branches so isolation still holds |

Never hard-fail on a missing plugin — degrade as above and report what you skipped.

## Process

### 1. Select the issues

Resolve from `arguments`: `all` open (default), a label, or explicit issue numbers.
List the matching open issues from the tracker. If the selection mode is ambiguous and
wasn't given, use **AskUserQuestion** to pick: *all open / by label / explicit list*. If
defaulting to all open, show the list and confirm before doing anything.

### 2. Disjointness gate (before any dispatch)

For each selected issue, read its body and lightly explore the target repo to predict the
**code regions** it will touch (files, dirs, tightly-coupled modules). Then partition the
issues into **strands**:

- Issues whose predicted regions are disjoint → each its own **singleton strand** (these fan out).
- Issues whose regions overlap → grouped into **one strand**, ordered by dependency.
- Uncertain overlap → put them in the same strand (serialize). Bias to serial.

The number of parallel agents equals the number of strands. Present the partition (which
issues are parallel, which are serialized together, and why) before dispatching.

### 3. Dispatch — one agent per strand, in parallel

Create one git worktree per strand and dispatch one agent per strand with the Agent/Task
tool, all in parallel:

```bash
git worktree add .worktrees/issue-<id> -b issue-<id> <base-branch>
```

Each agent is dispatched on **Opus** with **maximum reasoning effort** (dispatch params),
gets **only** the context it needs (its issue(s), its worktree path, the target repo's
`CLAUDE.md`/`CONTEXT.md`) — not this session's history — and runs the **per-agent brief**.
A singleton strand carries one issue; a multi-issue strand carries its issues in order, and
the agent does them **one at a time**, each producing its own commit(s) and its own PR.

The brief is **not** restated here — fill it from the template. Read
`assets/agent-brief.md` and substitute the placeholders per issue:

| Token | Value |
|-------|-------|
| `{{ISSUE_ID}}` | the issue number (e.g. `#41`) |
| `{{WORKTREE_PATH}}` | that strand's `.claude/worktrees/issue-<id>` path |
| `{{CONTEXT_FILE}}` | the file holding this issue's body + target-repo context |
| `{{MAX_REVIEW_PASSES}}` | `max_review_passes` from `config.json` (default 3) |

For a multi-issue (serialized) strand, fill and dispatch the brief once per issue in order;
later issues may be stacked (blocked-by) on the earlier PR, but each still ships its own PR
— never a shared one.

### 4. Collect & report

When the agents return, verify each issue produced exactly one PR and each worktree is
clean, then clean up the worktrees (`git worktree remove`). Report with the table below.

## Report format

| Issue | Strand | PR | Review passes | Leftover findings | Worktree |
|-------|--------|----|--------------|--------------------|----------|
| #41 | parallel | #88 | 2 | none | clean |
| #42 | serial w/ #57 | #89 | 3 | 1 (in PR body) | clean |

End with: total PRs opened (must equal issues worked), any strand that was serialized and
why, and anything that fell short of an iron guard. No shared merge — say so explicitly.
