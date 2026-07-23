"""Gemeinsame Basis der Regel-Reminder-Hooks (fokussiertes Modul, kein Hook selbst).

Bündelt bewusst das, wofür es im Repo noch **keine** Vorlage gab: Session-ID-Auflösung,
die Temp-State-Datei je Session, das Parsen der **einen** Signal-Tabelle aus
``.rules/README.md`` und die **eine** Entscheidungs-Funktion, die sowohl den informativen
``additionalContext`` als auch das Read-Gate speist. Die zwei dünnen Hook-Skripte
(``rule-reminder.py`` und ``rule-read-tracker.py``) importieren nur von hier.

Absichtlich getrennt vom bestehenden ``enforce-dependency-direction.py`` (das ist
zustandslos und single-purpose) — Fallstrick #5: neue Zuständigkeiten nicht in eine schlanke
Fremd-Datei häufen.

Stil analog zu ``auto-commit``: unveränderliche Records (``@dataclass(frozen=True)``),
Summentypen mit ``match`` statt Flag+Branch, reine Funktionen. Benötigt Python 3.10+
(``match``, PEP-604-Unions) — via ``uv run --script`` bereitgestellt.
"""

from __future__ import annotations

import json
import os
import re
import tempfile
from dataclasses import dataclass, field, replace
from typing import TypeAlias

# Drosselung der informativen Reminder — gilt NUR für bereits gelesene Regeln: je Ziel
# höchstens alle THROTTLE Signal-Edits. Ungelesene Regeln werden nicht gedrosselt, sondern
# blocken sofort (siehe decide).
THROTTLE = 3


# --- Signale: Summentyp Pfad- vs. Inhalts-Signal (illegale Kombination unbaubar) ------


@dataclass(frozen=True)
class PathSignal:
    """Matcht den repo-relativen Editpfad."""

    regex: re.Pattern[str]
    targets: tuple[str, ...]


@dataclass(frozen=True)
class ContentSignal:
    """Matcht den Datei-Inhalt nach dem Edit."""

    regex: re.Pattern[str]
    targets: tuple[str, ...]


# Ein Signal ist genau eine der beiden Formen — die Form bestimmt den Heuhaufen.
Signal: TypeAlias = PathSignal | ContentSignal


def _matches(signal: Signal, rel_path: str, content: str) -> bool:
    match signal:
        case PathSignal(regex):
            return regex.search(rel_path) is not None
        case ContentSignal(regex):
            return regex.search(content) is not None


# --- Session-State: unveränderliche Records + funktionale Updates ---------------------


@dataclass(frozen=True)
class TargetStat:
    """Drossel-Zähler EINES Ziels — unabhängig von Treffern auf andere Ziele."""

    edits: int = 0
    reminders: int = 0
    last_reminder_at_edit: int = 0


@dataclass(frozen=True)
class SessionState:
    """Der gesamte Session-State: gelesene Regeln + Zähler-Map je Ziel."""

    read_rules: frozenset[str] = frozenset()
    stats: dict[str, TargetStat] = field(default_factory=dict)


def with_read_rule(state: SessionState, rule: str) -> SessionState:
    return replace(state, read_rules=state.read_rules | {rule})


_SANITIZE_RE = re.compile(r"[^A-Za-z0-9_-]")


def session_id(payload: dict) -> str:
    """Session-ID aus dem stdin-Payload, Fallback Umgebungsvariable, sonst ``default``.

    NIE die PID des Hook-Prozesses verwenden — jeder Aufruf ist ein frischer Subprozess
    und könnte über Aufrufe hinweg nichts akkumulieren (Fallstrick #1).
    """
    raw = payload.get("session_id") or os.environ.get("CLAUDE_SESSION_ID") or "default"
    return _SANITIZE_RE.sub("_", str(raw))[:120] or "default"


def state_path(sid: str) -> str:
    return os.path.join(tempfile.gettempdir(), f"rule-reminder-{sid}.json")


def load_state(sid: str) -> SessionState:
    try:
        with open(state_path(sid), encoding="utf-8") as handle:
            raw = json.load(handle)
    except (OSError, ValueError):
        return SessionState()
    if not isinstance(raw, dict):
        return SessionState()
    read_rules = frozenset(str(r) for r in (raw.get("gelesene_regeln") or []))
    stats: dict[str, TargetStat] = {}
    for target, entry in (raw.get("pro_ziel") or {}).items():
        if isinstance(entry, dict):
            stats[str(target)] = TargetStat(
                edits=int(entry.get("edits", 0)),
                reminders=int(entry.get("reminders", 0)),
                last_reminder_at_edit=int(entry.get("last_reminder_at_edit", 0)),
            )
    return SessionState(read_rules=read_rules, stats=stats)


def save_state(sid: str, state: SessionState) -> None:
    """State best-effort schreiben; ein Schreibfehler darf den Hook nie brechen."""
    payload = {
        "gelesene_regeln": sorted(state.read_rules),
        "pro_ziel": {
            target: {
                "edits": stat.edits,
                "reminders": stat.reminders,
                "last_reminder_at_edit": stat.last_reminder_at_edit,
            }
            for target, stat in state.stats.items()
        },
    }
    try:
        with open(state_path(sid), "w", encoding="utf-8") as handle:
            json.dump(payload, handle, ensure_ascii=False)
    except OSError:
        pass


# --- Projekt-Root & Pfad-Normalisierung -----------------------------------------------


def project_dir(start: str | None = None) -> str:
    """Repo-Root: ``CLAUDE_PROJECT_DIR`` bevorzugt, sonst von hier aus hochlaufen."""
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env and os.path.isdir(os.path.join(env, ".rules")):
        return env
    current = os.path.dirname(os.path.abspath(start or __file__))
    while True:
        if os.path.isdir(os.path.join(current, ".rules")):
            return current
        parent = os.path.dirname(current)
        if parent == current:
            return os.getcwd()
        current = parent


def rel_to_project(path: str, root: str | None = None) -> str:
    """Editpfad repo-relativ, mit ``/`` als Trenner (leer, wenn kein Pfad)."""
    if not path:
        return ""
    root = root or project_dir()
    norm = os.path.abspath(path).replace("\\", "/")
    root_norm = os.path.abspath(root).replace("\\", "/").rstrip("/") + "/"
    return norm[len(root_norm):] if norm.startswith(root_norm) else path.replace("\\", "/")


def rule_rel(path: str) -> str | None:
    """``.rules/``-relativen Anteil eines Read-Pfades geben, sonst ``None``.

    ``…/.rules/common/testing.md`` -> ``common/testing.md``.
    """
    if not path:
        return None
    norm = path.replace("\\", "/")
    marker = "/.rules/"
    idx = norm.find(marker)
    if idx != -1:
        return norm[idx + len(marker):]
    if norm.startswith(".rules/"):
        return norm[len(".rules/"):]
    return None


def target_rule_path(target: str) -> str | None:
    """Aus einer Ziel-Zelle den führenden ``.rules/``-relativen Regel-Pfad ziehen.

    ``common/testing.md — „*_specs …"`` -> ``common/testing.md``. Das Read-Gate
    vergleicht darauf. ``None``, wenn das erste Token nicht wie ``*.md`` aussieht.
    """
    head = target.strip().split()
    if not head:
        return None
    token = head[0].split("#", 1)[0]
    return token if token.endswith(".md") else None


# --- Inhalt nach dem Edit rekonstruieren (PreToolUse läuft davor) ---------------------


def resulting_content(tool_name: str, tool_input: dict) -> str:
    """Datei-Inhalt NACH dem Edit rekonstruieren — wie ``enforce-dependency-direction.py``.

    Prüft nicht das Edit-Fragment, sondern den resultierenden Gesamtinhalt.
    """
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


# --- Signal-Tabelle laden (einzige Quelle der Wahrheit: .rules/README.md) -------------

_START = "<!-- SIGNAL-TABELLE:START -->"
_END = "<!-- SIGNAL-TABELLE:END -->"
# Zellen an nicht-escapten Pipes trennen (``\|`` bleibt Teil der Zelle).
_CELL_SPLIT_RE = re.compile(r"(?<!\\)\|")


def _split_row(row: str) -> list[str]:
    return [cell.strip().replace("\\|", "|") for cell in _CELL_SPLIT_RE.split(row)]


def _unwrap_backticks(text: str) -> str:
    stripped = text.strip()
    if len(stripped) >= 2 and stripped.startswith("`") and stripped.endswith("`"):
        return stripped[1:-1]
    return stripped


def load_signals(root: str | None = None) -> tuple[Signal, ...]:
    """Die Signal-Tabelle aus ``.rules/README.md`` in Signal-Records parsen.

    Defekte Zeilen (unkompilierbare Regex, fehlende Spalten, unbekannter Typ) werden
    still übersprungen — die Tabelle ist Menschentext und darf den Hook nie brechen.
    """
    root = root or project_dir()
    try:
        with open(os.path.join(root, ".rules", "README.md"), encoding="utf-8") as handle:
            text = handle.read()
    except OSError:
        return ()
    if _START not in text or _END not in text:
        return ()
    block = text.split(_START, 1)[1].split(_END, 1)[0]

    signals: list[Signal] = []
    for line in block.splitlines():
        row = line.strip()
        if not row.startswith("|"):
            continue
        cells = [cell for cell in _split_row(row) if cell]
        if len(cells) < 3:
            continue
        pattern = _unwrap_backticks(cells[1])
        # Die Ziel-Zelle ist als Ganzes backtick-umschlossen (Monospace in der README);
        # erst entklammern, dann an ``;`` in einzelne Ziele trennen — sonst kleben
        # Backticks an den Regel-Pfaden und brechen Dedup + Read-Gate-Vergleich.
        targets = tuple(t.strip() for t in _unwrap_backticks(cells[2]).split(";") if t.strip())
        if not pattern or not targets:
            continue
        try:
            regex = re.compile(pattern)
        except re.error:
            continue
        match cells[0].strip().lower():
            case "pfad":
                signals.append(PathSignal(regex, targets))
            case "inhalt":
                signals.append(ContentSignal(regex, targets))
            case _:
                continue  # überspringt Kopf-/Trennzeile automatisch
    return tuple(signals)


def evaluate_signals(rel_path: str, content: str, signals: tuple[Signal, ...]) -> tuple[str, ...]:
    """DIE eine Auswertung: getroffene Ziele (dedupliziert, reihenfolgestabil).

    Sowohl der informative Reminder als auch das Read-Gate rufen ausschließlich diese
    Funktion — nie eine zweite, unabhängige Heuristik (Fallstrick #3).
    """
    hit: list[str] = []
    for signal in signals:
        if _matches(signal, rel_path, content):
            for target in signal.targets:
                if target not in hit:
                    hit.append(target)
    return tuple(hit)


# --- Entscheidung: Summentyp Silent / Remind / Block ----------------------------------


@dataclass(frozen=True)
class Silent:
    """Kein Treffer oder alles gedrosselt/gelesen — Edit läuft still durch."""


@dataclass(frozen=True)
class Remind:
    """Informativer Reminder: nenne diese Ziele im ``additionalContext``."""

    targets: tuple[str, ...]


@dataclass(frozen=True)
class Block:
    """Regel-Gate: der Edit trifft eine Regel, deren Datei diese Session nie per Read
    geöffnet wurde -> harter Block (exit 2), sofort — auch in Turn 1."""

    rel_path: str
    unread_rules: tuple[str, ...]
    targets: tuple[str, ...]


Decision: TypeAlias = Silent | Remind | Block


def decide(
    rel_path: str,
    content: str,
    signals: tuple[Signal, ...],
    state: SessionState,
) -> tuple[Decision, SessionState]:
    """Reine Entscheidung + fortgeschriebener State. Keine I/O, keine Seiteneffekte.

    Harte Regel: Trifft der Edit **irgendeine** Regel, deren Datei diese Session noch nicht
    per Read geöffnet wurde, blockt der Hook **sofort** (auch in Turn 1) und nennt alle noch
    ungelesenen Regel-Dateien. Der erzwungene Read ist die bewusste Forcing-Function —
    Agenten haben den bloßen (ohnehin @-importierten) Reminder wiederholt überlesen.

    Sind **alle** betroffenen Regeln bereits gelesen, bleibt nur der informative Reminder,
    je Ziel gedrosselt (Fallstrick #4: eigener Zähler pro Ziel, unabhängig voneinander).
    """
    targets = evaluate_signals(rel_path, content, signals)
    if not targets:
        return Silent(), state

    unread_rules: list[str] = []
    block_targets: list[str] = []
    read_targets: list[str] = []
    for target in targets:
        rule = target_rule_path(target)
        if rule is not None and rule not in state.read_rules:
            block_targets.append(target)
            if rule not in unread_rules:
                unread_rules.append(rule)
        else:
            read_targets.append(target)  # Regel gelesen (oder nicht gatebar) -> nur Reminder

    # Sofort-Block, ohne State-Änderung: das Gate ist rein „Regel gelesen? ja/nein".
    if unread_rules:
        return Block(rel_path, tuple(unread_rules), tuple(block_targets)), state

    # Alle betroffenen Regeln gelesen -> gedrosselter informativer Reminder.
    stats = dict(state.stats)
    due: list[str] = []
    for target in read_targets:
        prev = stats.get(target, TargetStat())
        edits = prev.edits + 1
        is_due = prev.reminders == 0 or (edits - prev.last_reminder_at_edit) >= THROTTLE
        if is_due:
            stats[target] = TargetStat(edits=edits, reminders=prev.reminders + 1, last_reminder_at_edit=edits)
            due.append(target)
        else:
            stats[target] = replace(prev, edits=edits)

    new_state = replace(state, stats=stats)
    if due:
        return Remind(tuple(due)), new_state
    return Silent(), new_state
