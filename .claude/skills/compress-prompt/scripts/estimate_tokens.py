#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""estimate_tokens: rough token-count estimate for one text.

Requires Python 3.10+. The PEP 723 metadata block lets `uv run` provision a
suitable interpreter regardless of the system default.

Not a real tokenizer — Claude/GPT tokenizers differ and neither is bundled
here. Uses the common "~N characters per token" rule of thumb (N from the
sibling config.json, default 4) via the shared `token_estimate.estimate_tokens`
(see skills/_shared/token_estimate.py — also used by token-budget-audit).
Read one text from stdin or a file path and print a single integer (the
estimated token count) to stdout.

Usage:
  estimate_tokens.py [TEXT_PATH]      # omit the path to read from stdin

Exit 0 on success; 2 on a usage error (bad path / too many args).
"""

import json
import sys
from pathlib import Path
from typing import Any, cast

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "_shared"))
from token_estimate import estimate_tokens  # noqa: E402

CONFIG = Path(__file__).resolve().parent.parent / "config.json"
DEFAULT_CHARS_PER_TOKEN = 4


def _config() -> dict[str, Any]:
    try:
        return cast("dict[str, Any]", json.loads(CONFIG.read_text(encoding="utf-8")))
    except (OSError, ValueError):
        return {}


def read_input(argv: list[str]) -> str:
    match argv:
        case [] | ["-"]:
            return sys.stdin.read()
        case ["-h" | "--help"]:
            print(__doc__)
            sys.exit(0)
        case [path]:
            p = Path(path)
            if not p.is_file():
                print(f"estimate_tokens: error: file not found: {path}", file=sys.stderr)
                sys.exit(2)
            return p.read_text(encoding="utf-8")
        case _:
            print("estimate_tokens: error: pass at most one text path (or use stdin)", file=sys.stderr)
            sys.exit(2)


def main() -> int:
    chars_per_token = _config().get("chars_per_token", DEFAULT_CHARS_PER_TOKEN)
    print(estimate_tokens(read_input(sys.argv[1:]), chars_per_token))
    return 0


if __name__ == "__main__":
    sys.exit(main())
