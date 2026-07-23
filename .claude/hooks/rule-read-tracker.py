#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""PreToolUse-Hook (Matcher ``Read``): schneidet gelesene ``.rules/``-Dateien mit.

Rein passiv — schreibt den ``.rules/``-relativen Pfad jedes ``Read`` in den
Session-State und läuft IMMER mit ``exit 0`` durch (blockt nie ein Read). Der
Reminder-Hook liest diesen State später, um zu entscheiden, ob die zu einem Signal
passende Regel-Datei in der Session schon geöffnet wurde.

Empirisch bestätigt: ein ``Read``-Matcher-Hook sieht ``session_id`` und
``tool_input.file_path`` im stdin-Payload (die offene Frage aus Schritt 3 der Anleitung).
"""

from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _rule_reminder as rr  # noqa: E402


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    rule = rr.rule_rel((payload.get("tool_input") or {}).get("file_path") or "")
    if rule is None:
        sys.exit(0)

    try:
        sid = rr.session_id(payload)
        state = rr.load_state(sid)
        if rule not in state.read_rules:
            rr.save_state(sid, rr.with_read_rule(state, rule))
    except Exception:
        pass  # fail-open: ein Tracking-Fehler darf ein Read nie brechen
    sys.exit(0)


if __name__ == "__main__":
    main()
