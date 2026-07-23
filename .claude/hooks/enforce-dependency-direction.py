#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""PreToolUse-Hook: erzwingt „Abhängigkeiten zeigen nach innen".

Nur das einzige ``io``-Crate darf externe Fremd-Crates in seiner ``Cargo.toml`` binden;
``architecture`` (der Dependency-Richtungs-Validator, liegt außerhalb von ``crates/``) ist
ebenfalls ausgenommen. Alle anderen Crates — Domäne UND Adapter-Kern — ziehen ihre
Abhängigkeiten über öffentliche PORTS rein; eine externe ``[dependencies]``-Zeile dort wird
geblockt (exit 2).

Der Hook rekonstruiert den Datei-Inhalt NACH dem Edit (PreToolUse läuft davor) und prüft den
ganzen resultierenden Cargo.toml — so greift die Regel unabhängig davon, wie der Edit
zugeschnitten ist. Fehlt ein valider Payload oder scheitert das Lesen, wird NICHT geblockt
(fail-open, damit der Hook nie das Editieren bricht).

Stil analog zu ``auto-commit``: unveränderliche Records, ``match`` statt Flag+Branch, reine
Funktionen; benötigt Python 3.10+ (via ``uv run --script`` bereitgestellt).

Beleg: docs/prd/0001-bank-auswertung.md (Z.178-183), ADR-0001/ADR-0007; Memory
dependency-richtung-nach-innen. Der Test-Zwilling liegt in
architecture/tests/dependency_direction.rs (CI-Zeit; dieser Hook: Edit-Zeit).
"""

from __future__ import annotations

import json
import os
import re
import sys
from dataclasses import dataclass

# Fremd-Crates, die die Regel NICHT brechen (auch in Nicht-IO-Crates erlaubt).
SANCTIONED = frozenset({"serde", "serde_json", "thiserror", "static_assertions"})

# Crates, die binden dürfen: das einzige ``io`` + der Validator ``architecture``.
EXEMPT = frozenset({"io", "architecture"})

# Sektionen, deren Einträge Dependencies sind (auch target-spezifisch).
_DEP = r"(?:dependencies|dev-dependencies|build-dependencies)"
DEP_TABLE_RE = re.compile(r"^(?:target\.[^.]*\.)?" + _DEP + r"$")
DEP_ENTRY_RE = re.compile(r"^(?:target\.[^.]*\.)?" + _DEP + r"\.([A-Za-z0-9_-]+)$")
CRATE_CARGO_RE = re.compile(r"/crates/[^/]+/Cargo\.toml$")


# --- Inhalt nach dem Edit rekonstruieren ----------------------------------------------


def resulting_content(tool_name: str, tool_input: dict) -> str:
    """Den Datei-Inhalt NACH dem Edit rekonstruieren."""
    match tool_name:
        case "Write":
            return tool_input.get("content", "") or ""
        case "Edit":
            base = _read_base(tool_input.get("file_path", ""))
            old = tool_input.get("old_string", "")
            new = tool_input.get("new_string", "")
            return base.replace(old, new, 1) if old else base + new
        case "MultiEdit":
            content = _read_base(tool_input.get("file_path", ""))
            for edit in tool_input.get("edits", []) or []:
                old = edit.get("old_string", "")
                new = edit.get("new_string", "")
                content = content.replace(old, new, 1) if old else content + new
            return content
        case _:
            return _read_base(tool_input.get("file_path", ""))


def _read_base(path: str) -> str:
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except OSError:
        return ""


# --- Externe Dependencies aus dem Cargo.toml lesen ------------------------------------


def _is_local(text: str) -> bool:
    """path-Dep (workspace-intern) oder workspace-geerbte Dep -> erlaubt."""
    return bool(re.search(r"\bpath\s*=", text) or re.search(r"workspace\s*=\s*true", text))


def external_deps(content: str) -> tuple[str, ...]:
    """Externe (nicht sanktionierte, nicht path/workspace) Deps aus dep-Sektionen."""
    found: list[str] = []
    section: str | None = None
    entry_name: str | None = None  # innerhalb [dependencies.<name>]
    entry_is_local = False

    def flush_entry() -> None:
        nonlocal entry_name, entry_is_local
        if entry_name and not entry_is_local and entry_name not in SANCTIONED:
            found.append(entry_name)
        entry_name = None
        entry_is_local = False

    for raw in content.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if header := re.match(r"\[([^\]]+)\]", line):
            flush_entry()
            section = header.group(1).strip()
            sub = DEP_ENTRY_RE.match(section)
            entry_name = sub.group(1) if sub else None
            continue
        if entry_name is not None:
            if _is_local(line):
                entry_is_local = True
            continue
        if section is None or not DEP_TABLE_RE.match(section):
            continue
        if not (dep := re.match(r"([A-Za-z0-9_-]+)\s*=\s*(.+)", line)):
            continue
        name, value = dep.group(1), dep.group(2)
        if name in SANCTIONED or _is_local(value):
            continue
        found.append(name)

    flush_entry()
    return tuple(sorted(set(found)))


# --- Anwendbarer Edit + Entscheidung --------------------------------------------------


@dataclass(frozen=True)
class CargoEdit:
    """Ein regel-relevanter Cargo.toml-Edit: ein Nicht-EXEMPT-Crate unter ``crates/``."""

    crate_dir: str
    content: str


def applicable_edit(payload: dict) -> CargoEdit | None:
    """``None``, wenn der Edit die Regel gar nicht betrifft (kein Crate-Cargo.toml / EXEMPT)."""
    tool_input = payload.get("tool_input") or {}
    path = (tool_input.get("file_path") or "").replace("\\", "/")
    if not CRATE_CARGO_RE.search(path):
        return None
    crate_dir = os.path.basename(os.path.dirname(path))
    if crate_dir in EXEMPT:
        return None
    return CargoEdit(crate_dir, resulting_content(payload.get("tool_name", ""), tool_input))


def render_block(crate_dir: str, violations: tuple[str, ...]) -> str:
    deps = ", ".join(violations)
    return (
        f"BLOCKIERT — Dependency-Richtung verletzt (Crate `{crate_dir}`).\n"
        f"Externe Crate(s) im Nicht-IO-Layer: {deps}.\n\n"
        "Regel (UNUMSTÖSSLICH): Abhängigkeiten zeigen nach innen. Nur das einzige "
        "`io`-Crate bindet externe Fremd-Crates; alle anderen Layer — Domäne UND "
        "Adapter-Kern — ziehen ihre Abhängigkeiten über öffentliche PORTS rein.\n\n"
        f"Fix: `{deps}` hinter einen Port legen und die konkrete Nutzung in das "
        "`io`-Infra-Crate verschieben, das den Port erfüllt.\n"
        "Sanktioniert (nie geblockt): serde, serde_json, thiserror, static_assertions, "
        "sowie path-/workspace-Deps.\n"
        "Beleg: docs/prd/0001-bank-auswertung.md (Z.178-183), ADR-0001/ADR-0007; "
        "Memory dependency-richtung-nach-innen."
    )


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    edit = applicable_edit(payload)
    if edit is None:
        sys.exit(0)

    violations = external_deps(edit.content)
    if not violations:
        sys.exit(0)

    print(render_block(edit.crate_dir, violations), file=sys.stderr)
    sys.exit(2)  # PreToolUse: exit 2 blockt und zeigt stderr dem Modell.


if __name__ == "__main__":
    main()
