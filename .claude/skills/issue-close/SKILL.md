---
name: issue-close
description: "Closes an issue by setting status to 'closed' in its frontmatter and prepending a completion note to docs/issues/PROGRESS.md under ## Erledigt. Use when the user says 'close issue X', 'issue X is done', 'mark X as erledigt', 'issue X abschließen', or indicates an issue is complete."
arguments: "Issue ID or partial filename, optionally followed by a completion note — e.g. '0021' or '0021 Resolved via Option C (ADR-0019)'. If ID is omitted, ask which issue to close before proceeding. Note defaults to the issue title if not provided."
---

# Issue Close

Closes one issue: updates frontmatter `status`, prepends completion note to PROGRESS.md.

## config.json

```json
{
  "progress_file":        "docs/issues/PROGRESS.md",
  "done_section_heading": "## Erledigt"
}
```

## Process

### 1. Resolve the issue file and note
Parse `arguments`:
- Everything up to the first space (or the whole string) is the **ID/slug**.
- Everything after the first space is the **completion note** (may be empty).

Find the matching file:
```powershell
$arg = "<ID-part>"
Get-ChildItem "docs/issues" -Filter "*.md" |
  Where-Object { $_.Name -like "$arg*" -and $_.Name -ne "PROGRESS.md" }
```

If zero matches: stop — `"No issue found matching '<arg>'."`.
If multiple matches: list them and ask for a more specific ID.

### 2. Read and verify current status
Read the file with `bat "FILE" --paging=never`. Parse frontmatter.

- `status: closed` already → print `"Issue <ID> is already closed."` and stop.
- `status: open`, `status: blocked`, or field absent → proceed.

### 3. Update status in file
In the YAML frontmatter block (between the first two `---` markers):
- If `status:` line exists: replace it with `status: closed`.
- If `status:` is absent: add `status: closed` as the last field before the closing `---`.

Use the Edit tool with the exact line content for precision.

### 4. Prepend completion note to PROGRESS.md
Read `docs/issues/PROGRESS.md`. Locate the `## Erledigt` heading.
Insert a new entry directly below that heading:

```
- [x] **[<FILENAME>](<FILENAME>)** (<YYYY-MM-DD>)
  <note>
```

Where `<note>` is the completion note from `arguments`, or the issue `title` if none was provided.

Also replace the entire `Stand:` line at the top of the file with:
```
Stand: YYYY-MM-DD
```
(Trailing context after the date is discarded — the old context belongs to a previous update, not this one.)

If the `## Erledigt` heading is absent: stop and ask the user before creating it.

### 5. Report
```
Closed: 0021 — Materialise sibling sizes at group formation
  ✓ docs/issues/0021-materialise-sibling-sizes-at-group-formation.md  status → closed
  ✓ docs/issues/PROGRESS.md  completion note added, Stand: updated
```

## Done criterion

- Issue file: `status: closed` in frontmatter.
- PROGRESS.md: new entry prepended under `## Erledigt`, `Stand:` line updated to today.
