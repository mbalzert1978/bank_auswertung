#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""PreToolUse-Hook (Matcher ``Read``): blockt Volllesen großer Dateien ohne ``limit``.

Ein ``Read`` ohne explizites ``limit`` liest per Default bis zu 2000 Zeilen — bei großen
Dateien praktisch immer der ganze Inhalt, obwohl meist nur ein Ausschnitt gebraucht wird.
Das kostet Kontext, den man später (teurer) wieder wegkomprimieren muss.

Zustandslos, analog zu ``enforce-dependency-direction.py``: reine Funktion des aktuellen
Tool-Inputs, kein Session-State. Fail-open bei jedem Fehler (fehlende Datei, Binärdatei,
Permission) — der Hook darf nie ein Read brechen, das er nicht beurteilen kann.
"""

from __future__ import annotations

import json
import os
import sys

MAX_LINES = int(os.environ.get("READ_SIZE_GATE_MAX_LINES", "400"))


def _line_count(path: str) -> int | None:
    try:
        with open(path, encoding="utf-8") as handle:
            return sum(1 for _ in handle)
    except (OSError, UnicodeDecodeError):
        return None


def render_block(path: str, lines: int) -> str:
    return (
        f"BLOCKIERT — Read-Größen-Gate: `{path}` hat {lines} Zeilen (> {MAX_LINES}) und "
        "kein `limit` ist gesetzt — Read liest sonst per Default fast alles.\n\n"
        "Fix:\n"
        "  • Erst gezielt suchen (Grep) um die relevante Stelle zu finden, dann Read mit "
        "`offset`+`limit` nur auf den Ausschnitt.\n"
        f"  • Oder bewusst die ganze Datei: Read erneut mit explizitem `limit` (z. B. {lines}) "
        "aufrufen."
    )


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    tool_input = payload.get("tool_input") or {}
    path = tool_input.get("file_path") or ""
    if not path:
        sys.exit(0)

    if tool_input.get("limit"):
        sys.exit(0)  # explizit begrenzt -> bewusste Entscheidung, kein Gate

    lines = _line_count(path)
    if lines is None or lines <= MAX_LINES:
        sys.exit(0)

    print(render_block(path, lines), file=sys.stderr)
    sys.exit(2)


if __name__ == "__main__":
    main()
