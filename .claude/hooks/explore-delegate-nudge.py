#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""PreToolUse-Hook (Matcher ``Read|Grep|Glob|Agent``): erinnert an Delegation an Explore.

Zählt rohe Explorations-Calls (Read/Grep/Glob) je Session. Ab ``THRESHOLD`` seit dem
letzten Reset erinnert der Hook einmalig (``additionalContext``, kein Block — Exploration
vs. gezielter Read ist eine Ermessensfrage, ein harter Block hätte zu viele False
Positives). Jeder ``Agent``-Aufruf setzt den Zähler zurück (Delegation hat gewirkt).

State + Entscheidung gespiegelt von ``_rule_reminder.py``: unveränderlicher State-Record
plus eine reine ``decide``-Funktion, die einen Decision-Summentyp zurückgibt — ``main()``
matcht nur noch darauf, statt Flags aus einem try-Block herauszuschleusen.

Fail-open bei jedem Fehler. State liegt wie bei ``rule-reminder`` im Temp-Verzeichnis, je
Session-ID.
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
from dataclasses import dataclass, replace
from typing import TypeAlias

THRESHOLD = int(os.environ.get("EXPLORE_NUDGE_THRESHOLD", "6"))


# --- State + Entscheidung: unveränderlicher Record + Summentyp ------------------------


@dataclass(frozen=True)
class NudgeState:
    count: int = 0
    last_reminder_at: int = 0


@dataclass(frozen=True)
class Silent:
    """Kein Explorations-Call, ein Agent-Reset, oder Schwelle noch nicht erreicht."""


@dataclass(frozen=True)
class Remind:
    """Schwelle erreicht -> additionalContext mit diesem Zähler emittieren."""

    count: int


Decision: TypeAlias = Silent | Remind


def decide(tool_name: str, state: NudgeState) -> tuple[Decision, NudgeState]:
    """Reine Entscheidung + fortgeschriebener State. Keine I/O, keine Seiteneffekte."""
    match tool_name:
        case "Agent":
            return Silent(), NudgeState()  # Delegation hat gewirkt -> Reset
        case "Read" | "Grep" | "Glob":
            bumped = replace(state, count=state.count + 1)
            if bumped.count - bumped.last_reminder_at >= THRESHOLD:
                return Remind(bumped.count), replace(bumped, last_reminder_at=bumped.count)
            return Silent(), bumped
        case _:
            return Silent(), state


# --- Session-State: Temp-Datei je Session-ID -------------------------------------------


def _session_id(payload: dict) -> str:
    raw = payload.get("session_id") or os.environ.get("CLAUDE_SESSION_ID") or "default"
    safe = "".join(c if c.isalnum() or c in "_-" else "_" for c in str(raw))
    return safe[:120] or "default"


def _state_path(sid: str) -> str:
    return os.path.join(tempfile.gettempdir(), f"explore-nudge-{sid}.json")


def _load(sid: str) -> NudgeState:
    try:
        with open(_state_path(sid), encoding="utf-8") as handle:
            raw = json.load(handle)
    except (OSError, ValueError):
        return NudgeState()
    if not isinstance(raw, dict):
        return NudgeState()
    return NudgeState(
        count=int(raw.get("count", 0)),
        last_reminder_at=int(raw.get("last_reminder_at", 0)),
    )


def _save(sid: str, state: NudgeState) -> None:
    try:
        with open(_state_path(sid), "w", encoding="utf-8") as handle:
            json.dump({"count": state.count, "last_reminder_at": state.last_reminder_at}, handle)
    except OSError:
        pass


def _emit_context(text: str) -> None:
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": text,
        }
    }))


def render_reminder(count: int) -> str:
    return (
        f"🔍 Explore-Nudge (bank_auswertung) — {count} rohe Read/Grep/Glob-Calls in dieser "
        "Session ohne Agent-Delegation. Für breite Codebase-Exploration: den Agent-Tool-"
        "Subagenten (subagent_type=Explore) nutzen — durchsucht in eigenem Kontext, nur die "
        "Zusammenfassung kommt zurück. Für gezielte Einzel-Lookups (bekannter Pfad/Symbol) "
        "bleibt Read/Grep direkt richtig."
    )


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    try:
        sid = _session_id(payload)
        decision, new_state = decide(payload.get("tool_name", ""), _load(sid))
        _save(sid, new_state)
    except Exception:
        sys.exit(0)

    match decision:
        case Silent():
            sys.exit(0)
        case Remind(count):
            _emit_context(render_reminder(count))
            sys.exit(0)


if __name__ == "__main__":
    main()
