<!--
  HANDOFF DOCUMENT SKELETON for the handoff skill.

  Fill every slot from the current conversation so a fresh agent can continue the
  work cold. Rules that ride along with the slots:

  - DO NOT duplicate content already captured elsewhere (PRDs, plans, ADRs, issues,
    commits, diffs). Reference it by path or URL under References instead.
  - REDACT secrets — API keys, passwords, tokens, PII. Never paste them in.
  - If the invocation arguments named what the next session is for, let that focus
    shape Context and Open Items (lead with what serves that goal).
  - Drop a section's body if the conversation genuinely has nothing for it; keep the
    heading only when it carries content.
-->

# Handoff: {one-line subject of the work}

## Context

{Why this work exists and where it stands. Enough for a cold agent to orient:
the goal, the current state, and — if the invocation named a focus for the next
session — what that next session is meant to achieve.}

## Work Done

{What has already happened this session — decisions made, code written, things
tried and ruled out. Summarise; reference artifacts by path/URL rather than
repeating their content.}

## Open Items

{What still needs doing, in priority order. Concrete next steps, blockers, and any
open questions the next agent must resolve.}

## Suggested Skills

{Skills the next agent should invoke to continue, each with a one-line why.}

- `<skill-name>` — {when/why to reach for it}

## References

{Artifacts that already hold the detail — do not duplicate them, point to them.}

- `<path or URL>` — {what it is}
