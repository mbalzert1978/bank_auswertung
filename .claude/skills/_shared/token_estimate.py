"""Shared rough token-count estimate, used by compress-prompt and token-budget-audit.

Not a real tokenizer — Claude/GPT tokenizers differ and neither is bundled here.
Uses the common "~N characters per token" rule of thumb, accurate enough to
compare a before/after pair or flag a size outlier, not to bill against.
`chars_per_token` is supplied by the caller (each skill's own config.json) —
no default lives here, so there is exactly one place per skill that sets it.

Single source of truth. This file lives in skills/_shared/; each consuming
skill reaches it through a relative symlink at its own scripts/token_estimate.py
(same pattern as skills/_shared/inventory.py — see .claude/skills/CLAUDE.md).
Edit it HERE, never a symlink.
"""


def estimate_tokens(text: str, chars_per_token: int) -> int:
    return max(1, round(len(text) / chars_per_token)) if text else 0
