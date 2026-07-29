# Prompt-Templates — multi-agent-thermo-nuclear-review

Drei Blocks, vom Workflow-Script (`scripts/multi_agent_review.js`) per Platzhalter befüllt.
`scripts/prepare_args.py` parst genau die drei `## HEADER` / `## FINDER` / `## VERIFIER`
Fenced-Blocks heraus und reicht sie per `args.templates` durch.

Platzhalter:
- `HEADER`: `{{scope}}`, `{{guardrails}}`, `{{thermoStandardsPath}}`
- `FINDER`: `{{header}}`, `{{lens}}`, `{{angle}}`
- `VERIFIER`: `{{header}}`, `{{lens}}`, `{{candidates}}`

## HEADER

```text
Review the code at: {{scope}}
Open and read the files yourself — every cited line number must be accurate, so read before you judge.

Apply the FULL review standard of the thermo-nuclear-code-quality-review skill: read the rubric at
{{thermoStandardsPath}} and hold the code to its "Non-Negotiable Additional Standards", "What to Flag
Aggressively", and "Approval Bar". This is an extremely strict, structure-first maintainability review —
be ambitious about code-judo restructurings, missing abstractions, and making invalid states unrepresentable.

PROJECT GUARDRAILS (deliberate, documented project decisions — proposing to "fix" any of these is a FALSE POSITIVE):
{{guardrails}}
The codebase is already high-quality and heavily refactored. Every finding must clear a HIGH bar: a real,
concrete, actionable weakness — not a style preference, and not something the guardrails above already settle.

Cite the EXACT repo-relative file path and line number for every finding.
```

## FINDER

```text
{{header}}

You are reviewing under ONE lens: {{lens}}.
Primary angle for this pass: {{angle}}

Return your top 2-3 findings for the {{lens}} lens overall — use the angle as your entry point, but report
the strongest {{lens}} issues you actually find. Each must be a concrete, located weakness that clears the
high bar and does not collide with the project guardrails.
```

## VERIFIER

```text
{{header}}

You are the ADVERSARIAL VERIFIER for the {{lens}} lens. Below are candidate findings from several independent
finders. Your job:
(1) OPEN each cited file:line and confirm the finding is REAL and the line is right;
(2) REJECT any that is a false positive, a mere style preference, or collides with a PROJECT GUARDRAIL;
(3) if NOTHING survives — every candidate is a false positive, a style preference, or a guardrail collision —
    return the explicit "nothing real" state: outcome = { found: false } (plus the rejected list inside outcome).
    Do NOT invent a finding;
(4) otherwise return outcome = { found: true, ... } and, of the survivors, pick the SINGLE highest cost/benefit
    finding (most impact for least change/risk); write it as Befund (one German line incl. file:line) + Verbesserung
    (1-3 German lines, concrete, NO rewrite-code dumps) + Priorität (hoch/mittel/niedrig).
Set confidence=confirmed only if you personally re-read the code and it holds; plausible otherwise.
List what you rejected and why (brief), in both cases. Be skeptical: this codebase is already heavily refactored,
so default to rejecting — but reject honestly to found=false rather than manufacturing a weak finding.

CANDIDATES:
{{candidates}}
```
