#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""strip_dashes: deterministic dash-family removal for one text variant.

Requires Python 3.10+ (match statements). The PEP 723 metadata block above lets
`uv run` provision a suitable interpreter regardless of the system default —
invoke via `uv run scripts/strip_dashes.py` (see SKILL.md).

This is the producer's deterministic safety net, NOT a gate: the generator is
already told to write dash-free text (Producer step 2), so on real input this
script usually changes nothing. It runs exactly once per variant, has no loop,
never rejects, never regenerates, and always succeeds — it just guarantees no
dash slips through. Read one variant from stdin (or a file path) and write it to
stdout with the dash family removed; the only other change is that runs of
in-line spaces/tabs are collapsed to one (the gap a removed dash leaves, and any
pre-existing run). Newlines are preserved.

Removes the whole dash family — U+002D `-`, U+2010 `‐`, U+2011 `‑`, U+2012 `‒`,
U+2013 `–`, U+2014 `—`, U+2015 `―`, U+2212 `−` — by two rules:

  glue   a dash between two non-space chars ("Code-Quality")  -> one space
  gap    a space-surrounded dash ("Wort — Wort")              -> removed, the
         doubled space it leaves collapsed back to one

Usage:
  strip_dashes.py [TEXT_PATH]      # omit the path to read from stdin

Exit 0 on success (cleanup cannot fail); 2 on a usage error (bad path / too many
args).
"""

import re
import sys
from pathlib import Path

# The full dash family this skill forbids, in the spec's order.
DASHES = "-‐‑‒–—―−"
_CLASS = "[" + DASHES + "]"

# A dash gluing two non-space tokens together ("Code-Quality") — becomes a space
# so the two words do not merge.
_GLUE = re.compile(r"(?<=\S)" + _CLASS + r"(?=\S)")
# Any dash that survives the glue pass (space-adjacent or at a boundary) — dropped.
_GAP = re.compile(_CLASS)
# Runs of in-line spaces/tabs (e.g. the gap a removed dash leaves) — collapsed to
# one. Newlines are preserved so multi-line variants keep their shape.
_RUNS = re.compile(r"[ \t]{2,}")


def strip_dashes(text: str) -> str:
    """Return `text` with the whole dash family removed by the glue/gap rules."""
    text = _GLUE.sub(" ", text)
    text = _GAP.sub("", text)
    return _RUNS.sub(" ", text)


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
                print(f"strip_dashes: error: file not found: {path}", file=sys.stderr)
                sys.exit(2)
            return p.read_text(encoding="utf-8")
        case _:
            print("strip_dashes: error: pass at most one text path (or use stdin)", file=sys.stderr)
            sys.exit(2)


def main() -> int:
    sys.stdout.write(strip_dashes(read_input(sys.argv[1:])))
    return 0


if __name__ == "__main__":
    sys.exit(main())
