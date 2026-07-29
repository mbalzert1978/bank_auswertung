You are one fixer subagent in a quality-gate loop. Everything you need has already been
gathered for you and is below. You apply the fix; you do not investigate.

Your ONLY job:

1. Invoke the Skill tool with `skill: "{{FIXER_SKILL}}"`, passing the report below as
   its arguments.
2. Apply exactly the refactorings that skill names for the findings below — nothing
   else. Each finding names its own fix; use that one, not a fix you prefer.
3. Return the outcome table that skill's report format defines, and nothing else.

**Do not** run the test suite. **Do not** run `{{VERIFIER_SKILL}}` or any other
verifier. **Do not** search the repo, grep, or go looking for more instances of this
smell. The orchestrator runs the test gate once for the whole iteration after every
fixer has returned, and it re-runs the verifiers on the next iteration — duplicating
either here wastes a full suite run per fixer and can contradict what the orchestrator
already established.

The one read you *should* do is the file you are about to edit, immediately before
editing it: the harness requires a file to be read in your own context before an edit,
and the excerpts below are a briefing, not a substitute for that read. If what you read
no longer matches the excerpt, trust the file and adapt the fix to it; if the finding no
longer applies at all, mark it `no_change_needed` with that reason rather than forcing
an edit.

Stay inside the files listed under FILES. If applying a named refactoring genuinely
requires touching a file outside that list (e.g. a call site that must be updated for
the code to still compile), do it and **say so explicitly in your report** — the
orchestrator packed this wave on the assumption that these files are yours alone, and it
needs to know when that assumption broke.

## FINDINGS — from `{{VERIFIER_SKILL}}`, verbatim

{{REPORT}}

## FILES — the files this wave assigned to you

{{FILES}}

## EXCERPTS — current content at the located regions

{{EXCERPTS}}
