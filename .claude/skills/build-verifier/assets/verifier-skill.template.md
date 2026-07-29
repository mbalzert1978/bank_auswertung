---
name: <kebab-case-name>
description: <what it verifies, in one statement>. Use when <trigger phrases that should fire it>.
arguments: <the artifact or output to verify, passed in at invocation>
---

# <Title>

Verify <the one thing> and return an objective verdict: <Pass/Fail | a grade out of 10>.

## Verdict

<For Pass/Fail: the exact condition that makes it PASS, and what makes it FAIL.>
<For a grade: the /10 rubric — what earns full marks and what each band means.>

## Criteria

Each criterion is testable and feeds the verdict.

1. **<criterion>** — <how it is checked (the yes/no test or measurable threshold)> — <correctness | quality>
2. **<criterion>** — <how it is checked> — <correctness | quality>

## External data

<What the check needs from outside and which tool pulls it in (web search, an API, file read, an MCP server). Write "none — self-contained" if the artifact carries everything the check needs. Note any prerequisite that is not yet wired up.>

## Process

1. <pull the external data / read the artifact under test>
2. <run each criterion check, recording its result>
3. <combine the results into the verdict>

## Output format

<The exact verdict the skill emits, read at a glance. For Pass/Fail:>

**Verdict: PASS** — <one-line reason>

| Criterion | Result |
| --------- | ------ |
| <criterion> | Pass / Fail |

<For a grade, emit the score and the per-criterion breakdown that sums to it:>

**Grade: <n>/10**

| Criterion | Points | Notes |
| --------- | ------ | ----- |
