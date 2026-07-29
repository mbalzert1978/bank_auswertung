---
name: issue-status
description: "Scans docs/issues/ for all issue files, extracts YAML frontmatter status fields, and prints a grouped status table (blocked / open / closed) to chat. Use when the user asks 'was ist offen', 'what's left', 'issue overview', or wants a summary of issue states."
---

# Issue Status

One-shot scan of `docs/issues/` → status table in chat. No files written.

## config.json

```json
{
  "issue_dir":     "docs/issues",
  "progress_file": "PROGRESS.md"
}
```

The scanned directory and the excluded progress filename come from here, not the script body.

## Process

### 1. Collect and parse issue files
Run the bundled script from the repo root:
```powershell
uv run .claude\skills\issue-status\scripts\list-issues.py
```

The script reads all `.md` files in the configured `issue_dir` except the `progress_file`, extracts
`id`, `title`, and `status` from each file's YAML frontmatter, and outputs a JSON array.
Files with no parseable frontmatter are emitted with `status: "(malformed)"` so they
appear in the output rather than being silently dropped; their `id` falls back to the
filename.

### 2. Group and sort
From the JSON, group by `status`. Order the groups:

1. `blocked`
2. `open`
3. `closed`
4. any other status value present (e.g. `done`, `resolved`) — alphabetically, after `closed`
5. `(malformed)` — no parseable frontmatter (shown last, flagged)

Sort within each group by `id` ascending. Every row from the JSON must land in exactly
one group — never drop a status.

### 3. Print report
Output a single markdown table:

```
## Issue Status — YYYY-MM-DD

| ID   | Title                                             | Status   |
|------|---------------------------------------------------|----------|
| 0003 | Separate SFTP connection budgets…                 | blocked  |
| 0006 | Outbox dead-letter queue                          | open     |
| 0015 | Outcome-DU placement decision                     | closed   |

**blocked: N** | **open: M** | **closed: K** | … | total: T
```

The summary line carries one `**status: count**` segment per group present, then `total: T`.

If the issue dir is empty or the script returns no results: print
`"No issues found in docs/issues/."` and stop.

## Done criterion

Table is printed to chat.
