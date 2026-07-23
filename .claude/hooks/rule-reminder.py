#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""PreToolUse-Hook (Matcher ``Write|Edit|MultiEdit``): erinnert am Edit an die passende Regel.

Wertet die Signal-Tabelle aus ``.rules/README.md`` über die **eine** gemeinsame Funktion
in ``_rule_reminder`` aus. Trifft ein Edit ein Signal, nennt der ``additionalContext``
**konkret** die betroffene Regel-Datei + Abschnitt — nicht „siehe Tabelle".

Die Regeln sind via ``@``-Import ohnehin in jeder Session geladen (CLAUDE.md); dieser Hook
holt die am Edit relevante Regel just-in-time wieder hoch. Fail-open bei jedem Fehler.

Blockt (``exit 2``) **sofort** — auch in Turn 1 — sobald der Edit eine Regel trifft, deren
Datei in dieser Session noch nicht per ``Read`` geöffnet wurde: der Block erzwingt bewusst,
dass der Agent die konkrete Regel-Datei aufschlägt, statt die (ohnehin @-importierte) Regel
zu überlesen. Ist die betroffene Regel gelesen, folgt nur noch der gedrosselte informative
Reminder (``exit 0``).
"""

from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import _rule_reminder as rr  # noqa: E402


def _emit_context(text: str) -> None:
    """``additionalContext`` im PreToolUse-JSON-Format auf stdout (exit 0)."""
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": text,
        }
    }))


def render_reminder(targets: tuple[str, ...]) -> str:
    bullets = "\n".join(f"  • {t}" for t in targets)
    return (
        "📐 Regel-Reminder (bank_auswertung) — dieser Edit berührt Regeln, die jetzt gelten:\n"
        f"{bullets}\n"
        "Quelle: .rules/README.md (Signal-Tabelle). Die Regeln sind via @-Import bereits "
        "im Kontext; hier just-in-time die am Edit relevanten."
    )


def render_block(rel_path: str, unread_rules: tuple[str, ...], targets: tuple[str, ...]) -> str:
    rules = "\n".join(f"  • .rules/{r}" for r in unread_rules)
    hits = "\n".join(f"  • {t}" for t in targets)
    return (
        f"BLOCKIERT — Regel-Gate: `{rel_path}` berührt Regel(n), deren Datei(en) du in "
        "dieser Session noch nicht per `Read` geöffnet hast:\n"
        f"{rules}\n\n"
        "Betroffene Regel(n):\n"
        f"{hits}\n\n"
        "Fix: die genannte(n) Regel-Datei(en) zuerst per `Read` öffnen (klärt das Gate für "
        "die Session), dann den Edit erneut ausführen. Die Regeln sind zwar via @-Import "
        "geladen — der Read erzwingt bewusst, dass du genau diese Regel vor dem Edit ansiehst."
    )


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    tool_input = payload.get("tool_input") or {}
    path = tool_input.get("file_path") or ""

    try:
        root = rr.project_dir()
        rel_path = rr.rel_to_project(path, root)
        content = rr.resulting_content(payload.get("tool_name", ""), tool_input)
        signals = rr.load_signals(root)
        sid = rr.session_id(payload)
        decision, new_state = rr.decide(rel_path, content, signals, rr.load_state(sid))
        rr.save_state(sid, new_state)
    except Exception:
        sys.exit(0)  # fail-open: der Hook bricht nie das Editieren

    match decision:
        case rr.Silent():
            sys.exit(0)
        case rr.Remind(targets):
            _emit_context(render_reminder(targets))
            sys.exit(0)
        case rr.Block(rel, unread_rules, targets):
            print(render_block(rel, unread_rules, targets), file=sys.stderr)
            sys.exit(2)  # PreToolUse: exit 2 blockt und zeigt stderr dem Modell


if __name__ == "__main__":
    main()
