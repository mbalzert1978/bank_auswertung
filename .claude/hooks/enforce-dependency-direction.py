#!/usr/bin/env python3
"""PreToolUse-Hook: erzwingt "Abhängigkeiten zeigen nach innen".

Nur IO-Layer-Crates (Verzeichnisname endet auf ``_io``) dürfen externe Fremd-Crates
in ihrer ``Cargo.toml`` binden. Alle anderen Crates — Domäne UND Adapter-Kern —
ziehen ihre Abhängigkeiten über öffentliche PORTS rein; eine externe
``[dependencies]``-Zeile dort wird geblockt (exit 2).

Der Hook rekonstruiert den Datei-Inhalt NACH dem Edit (PreToolUse läuft davor) und
prüft den ganzen resultierenden Cargo.toml — so greift die Regel unabhängig davon,
wie der Edit zugeschnitten ist. Fehlt ein valider Payload oder scheitert das Lesen,
wird NICHT geblockt (fail-open, damit der Hook nie das Editieren bricht).

Beleg: docs/prd/0001-bank-auswertung.md (Z.178-183), ADR-0001; Memory
dependency-richtung-nach-innen.
"""

import json
import os
import re
import sys

# Fremd-Crates, die die Regel NICHT brechen (auch in Nicht-IO-Crates erlaubt).
SANCTIONED = {"serde", "serde_json", "thiserror", "static_assertions"}

# Sektionen, deren Einträge Dependencies sind (auch target-spezifisch).
_DEP = r"(?:dependencies|dev-dependencies|build-dependencies)"
DEP_TABLE_RE = re.compile(r"^(?:target\.[^.]*\.)?" + _DEP + r"$")
DEP_ENTRY_RE = re.compile(r"^(?:target\.[^.]*\.)?" + _DEP + r"\.([A-Za-z0-9_-]+)$")

CRATE_CARGO_RE = re.compile(r"/crates/[^/]+/Cargo\.toml$")


def resulting_content(tool_name, tool_input):
    """Den Datei-Inhalt NACH dem Edit rekonstruieren."""
    if tool_name == "Write":
        return tool_input.get("content", "")
    path = tool_input.get("file_path", "")
    try:
        with open(path, "r", encoding="utf-8") as handle:
            base = handle.read()
    except OSError:
        base = ""
    if tool_name == "Edit":
        old = tool_input.get("old_string", "")
        new = tool_input.get("new_string", "")
        return base.replace(old, new, 1) if old else base + new
    if tool_name == "MultiEdit":
        content = base
        for edit in tool_input.get("edits", []) or []:
            old = edit.get("old_string", "")
            new = edit.get("new_string", "")
            content = content.replace(old, new, 1) if old else content + new
        return content
    return base


def _is_local(text):
    """path-Dep (workspace-intern) oder workspace-geerbte Dep -> erlaubt."""
    return bool(re.search(r"\bpath\s*=", text) or re.search(r"workspace\s*=\s*true", text))


def external_deps(content):
    """Externe (nicht sanktionierte, nicht path/workspace) Deps aus dep-Sektionen."""
    found = []
    section = None
    entry_name = None            # innerhalb [dependencies.<name>]
    entry_is_local = False

    def flush_entry():
        nonlocal entry_name, entry_is_local
        if entry_name and not entry_is_local and entry_name not in SANCTIONED:
            found.append(entry_name)
        entry_name = None
        entry_is_local = False

    for raw in content.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        header = re.match(r"\[([^\]]+)\]", line)
        if header:
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
        dep = re.match(r"([A-Za-z0-9_-]+)\s*=\s*(.+)", line)
        if not dep:
            continue
        name, value = dep.group(1), dep.group(2)
        if name in SANCTIONED or _is_local(value):
            continue
        found.append(name)

    flush_entry()
    return found


def main():
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    tool_input = payload.get("tool_input") or {}
    path = (tool_input.get("file_path") or "").replace("\\", "/")

    # Nur Crate-Cargo.toml, nicht das Workspace-Root-Cargo.toml.
    if not CRATE_CARGO_RE.search(path):
        sys.exit(0)

    crate_dir = os.path.basename(os.path.dirname(path))
    if crate_dir.endswith("_io"):
        sys.exit(0)  # IO-Layer darf binden.

    try:
        content = resulting_content(payload.get("tool_name", ""), tool_input)
    except Exception:
        sys.exit(0)

    violations = sorted(set(external_deps(content)))
    if not violations:
        sys.exit(0)

    deps = ", ".join(violations)
    print(
        f"BLOCKIERT — Dependency-Richtung verletzt (Crate `{crate_dir}`).\n"
        f"Externe Crate(s) im Nicht-IO-Layer: {deps}.\n\n"
        "Regel (UNUMSTÖSSLICH): Abhängigkeiten zeigen nach innen. Nur der IO-Layer "
        "(`*_io`-Crate) bindet externe Fremd-Crates; alle anderen Layer — Domäne UND "
        "Adapter-Kern — ziehen ihre Abhängigkeiten über öffentliche PORTS rein.\n\n"
        f"Fix: `{deps}` hinter einen Port legen und die konkrete Nutzung in ein "
        "`*_io`-Infra-Crate verschieben, das den Port erfüllt.\n"
        "Sanktioniert (nie geblockt): serde, serde_json, thiserror, static_assertions, "
        "sowie path-/workspace-Deps.\n"
        "Beleg: docs/prd/0001-bank-auswertung.md (Z.178-183), ADR-0001; "
        "Memory dependency-richtung-nach-innen.",
        file=sys.stderr,
    )
    sys.exit(2)  # PreToolUse: exit 2 blockt und zeigt stderr dem Modell.


if __name__ == "__main__":
    main()
