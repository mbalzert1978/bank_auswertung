---
name: handoff
description: Compact the current conversation into a handoff document for another agent to pick up.
arguments: Optional. What the next session will be used for / its focus — used to tailor the handoff. If omitted, summarise the conversation as-is.
---

Write a handoff document summarising the current conversation so a fresh agent can continue the work. Save to the temporary directory of the user's OS — not the current workspace.

Fill the bundled skeleton rather than constructing the document from scratch:

```
<this-skill's-base-dir>/assets/handoff-template.md
```

Read it and fill every slot from the current conversation. The template is the single source of the document's section structure (Context / Work Done / Open Items / Suggested Skills / References) and carries the point-of-use reminders for each — the rules below restate the load-bearing ones:

- Include a **Suggested Skills** section that names skills the next agent should invoke.
- Do not duplicate content already captured in other artifacts (PRDs, plans, ADRs, issues, commits, diffs). Reference them by path or URL instead.
- Redact any sensitive information, such as API keys, passwords, or personally identifiable information.
- If the user passed arguments, treat them as a description of what the next session will focus on and tailor the doc accordingly.
