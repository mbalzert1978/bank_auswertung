---
name: done
description: "End-of-session wrapper. Runs /reflect for experience extraction, prints a session summary, updates progress tracking, and flags outstanding work. Use when finishing a session."
---

# Done — Session End Skill

Wraps up the current session with experience extraction, progress tracking, and status reporting.

## Step 1: Run /reflect

Invoke the `/reflect` skill. This extracts experiences from the conversation, persists them to memory, and runs the decay sweep.

Wait for `/reflect` to complete before proceeding.

## Step 2: Session Summary

Read `assets/session-summary.md`, fill its placeholders, and print the result.

Derive the placeholder values from:
- Tasks completed during the session
- Files created or modified (check git status/diff if available)
- Any outstanding tasks still pending
- Any blockers or questions raised but not resolved

Placeholder tokens (shared with Step 3):
- `{{BUILT}}` — bulleted list of features/changes completed
- `{{IN_PROGRESS}}` — bulleted list of unfinished work
- `{{BLOCKED}}` — bulleted list of blockers (if any)
- `{{NEXT}}` — bulleted list of suggested next steps

## Step 3: Update Progress File

Read `assets/progress.md`, fill its placeholders with the same values as Step 2 plus `{{DATE}}` (today, `YYYY-MM-DD`), and write the result to a `progress.txt` file in the project root, replacing any existing content.

## Step 4: Flag Outstanding Work

If any task tracking files have unchecked items, remind the user:
> "There are N unchecked items in the task list. Review before next session?"

## Rules
- This skill is the GUARANTEED trigger for `/reflect`. Any hooks are best-effort.
- Keep the summary terse — this is end-of-session, not a report.
- Do not prompt for confirmation — just run. The user typed `/done`, they want to wrap up.
