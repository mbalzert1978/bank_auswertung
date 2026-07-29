---
name: compress-prompt
description: Rewrite a prompt or text to use fewer tokens while preserving its meaning and intent, by applying a fixed set of token-reduction techniques (strip filler/politeness, cut redundant context, prefer structured over prose output, flag a missing output-length cap) and report the estimated before/after token count. Use when the user wants to "reduce tokens", "compress this prompt", "diesen Prompt token-sparender machen", "make this prompt cheaper", "kürze das für weniger Tokens".
arguments: The text/prompt to compress — inline text, or a path to a file containing it. If omitted, ask for it.
---

# Compress Prompt

Take a prompt or piece of text and hand back a version that costs fewer tokens to send, without changing what it asks for. That is the whole job — text in, cheaper text out, the same way every time.

This is **not** `refine-prompt`. `refine-prompt` optimizes for *quality* (clarity, structure, guardrails); this skill optimizes for *cost* (tokens in, tokens out). Run them independently, or `refine-prompt` first and this second if both are wanted — never merge the two jobs into one pass.

## ⛔ Iron rule

**Meaning is fixed.** Only wording, redundancy, and structure may change. Never drop a constraint, add a requirement, or reinterpret what's being asked. If a cut would change what the prompt means, don't make it — leave that part alone and note why in the report.

## Process

### 1. Get the text

- **Inline argument** → that text is the input.
- **A path** → read the file; its contents are the input.
- **Nothing passed** → ask for the text (or its path) in one short line.

Detect the input's language; the compressed output stays in that language.

### 2. Estimate the baseline

Run the bundled script to get the "before" figure — don't estimate by eye:

```bash
uv run .claude/skills/compress-prompt/scripts/estimate_tokens.py <path-to-text>
# or, for inline text: printf '%s' "<text>" | uv run .claude/skills/compress-prompt/scripts/estimate_tokens.py
```

It prints a single integer (chars ÷ `chars_per_token` from the bundled `config.json`, default 4). It's a rough estimate, not an exact tokenizer count — good enough to compare a before/after pair.

### 3. Apply the techniques

Walk the input against each technique below. Apply only the ones that genuinely fit; skip the rest — don't force a technique onto text it doesn't apply to.

1. **Strip filler and politeness.** "Could you please summarize this for me?" → "Summarize:". Remove hedging, throat-clearing, and pleasantries that carry no instruction.
2. **Cut redundant/repeated context.** If the surrounding conversation already established a fact, reference it ("Regarding the API design above, …") instead of restating it in full.
3. **Prefer structured output over prose**, when the consumer is a program or a quick scan is enough — ask for JSON, a bullet list, or a table instead of paragraphs. Skip this when the human-readable prose *is* the point.
4. **Flag a missing output-length cap.** If the prompt invites an open-ended answer and a shorter one would do, note that an explicit length constraint (e.g. "in 3 bullet points", "under 100 words", or a `max_tokens` setting at the API layer) would prevent an unnecessarily long response.
5. **Move stable background info out of the prompt.** If the prompt repeats context that rarely changes (tech stack, style rules, project layout), suggest it belongs in a persisted reference (a memory file, `CLAUDE.md`, or system prompt) instead of being retyped — but don't relocate it yourself; just flag it.

### 4. Estimate the result and report

Run `scripts/estimate_tokens.py` again on the compressed text for the "after" figure, then compute the percent reduction.

## Report format

Read the bundled template and fill it — don't re-derive the layout inline:

```
<this-skill's-base-dir>/assets/report-template.md
```

Fill `{{COMPRESSED_TEXT}}`, `{{BEFORE}}`/`{{AFTER}}`/`{{PERCENT}}` (from step 2 and 4), and each technique's `{{..._STATUS}}` (`yes` / `no` / `flagged` / `n/a`) and `{{..._NOTE}}` (one short line, or `—` if nothing to add).

Skip rows for techniques that plainly don't apply — don't pad the report. If the input is already lean, say so and return it unchanged rather than inventing cuts.
