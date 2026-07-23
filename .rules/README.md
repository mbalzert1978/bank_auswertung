# .rules — zentrale Regel- und Konventionsablage

Der **eine** Ort für die normativen Coding-Regeln und Konventionen dieses Repos.
[CLAUDE.md](../CLAUDE.md) bindet diese Dateien per `@`-Import ein (sie werden also jede
Session automatisch geladen) und behält selbst nur noch **Projekt-Narrativ, Struktur- und
Status-Kontext** (was das Projekt ist, welche Crates/Slices existieren, Baustand,
Kommando-Referenz, Baureihenfolge) — die *Regeln* leben hier.

Geschichtet:

- **[common/](common/)** — allgemeine, sprachunabhängige Regeln (Architektur-Prinzipien,
  Privacy-Invarianten, Domänenmodellierung, Testkultur, Sprach- und Ticket-Konvention,
  Memory-Abschaltung).
- **[rust/](rust/)** — Rust-spezifische Regeln (die Cargo-Gates, Rust-Idiome).
- **[crates/](crates/)** — Crate-spezifische Regeln (Dependency-Richtung, Kapselung).

Die maßgebliche Begründung jeder Regel liegt in den [ADRs](../docs/adr/); die Dateien
hier verweisen darauf, statt sie zu ersetzen.

## Regelformat (verbindlich)

Jede Regel ist eine atomare `##`-Einheit: eine **kurze normative Aussage**, gefolgt von
einem **Do:**- und einem **Don't:**-Beispiel in einem Code-Block. Codebeispiele in Rust
(Konfig-Regeln als `toml`-Ausschnitt, Prozess-Regeln in `bash` bzw. knapper Prosa); der
Don't-Block benennt die Gefahr per Inline-Kommentar. Muster:

````markdown
## Tokio als Async-Runtime

Für die Async-Runtime Tokio verwenden; `async-std` nur zur WASM-Kompatibilität.

Do:
```rust
#[tokio::main]
async fn main() {
    let daten = lade().await;
    verarbeite(daten).await;
}
```

Don't:
```rust
#[async_std::main] // zweite Runtime im Nicht-WASM-Build — fragmentiert das Async-Ökosystem
async fn main() {
    let daten = lade().await;
    verarbeite(daten).await;
}
```
````

Neue Regeln **immer** in diesem Format ergänzen.

## Geltungsbereich

Die Einträge hier sind **allgemeine Best-Practice-Regeln**; jede gilt nur dort, wo sie auf
die aktuelle Architektur **zutrifft** — nicht jede Regel ist auf jeden Teil des Repos
anwendbar. Zwei bewusst so belassene Fälle:

- **Async-Regeln** ([rust/nebenlaeufigkeit.md](rust/nebenlaeufigkeit.md)): Der aktuelle Stand
  / das MVP ist **synchron** (Outbox-Spine synchron, `sqlx`/async bewusst verworfen —
  [ADR-0014](../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)). Die Regeln
  greifen erst, **falls/sobald** async eingeführt wird.
- **SQL-Dialekt** ([sql/](sql/)): Die Beispiele sind allgemein (teils Postgres-Features wie
  `uuid_generate_v4`/Materialized Views). Die reale io-Persistenz ist **SQLite**; wo eine
  Regel zutrifft, sinngemäß auf den SQLite-Dialekt übertragen.

## Signal-Tabelle (Regel-Reminder-Hook)

**Einzige Quelle der Wahrheit** dafür, welche Datei-Edits an welche Regel erinnern. Die
`@`-Importe in [CLAUDE.md](../CLAUDE.md) laden zwar *alle* Regeln in jede Session — diese
Tabelle holt in langen Sitzungen die **eine** am Edit relevante Regel just-in-time wieder
hoch, direkt bevor der Edit läuft.

Der Hook [`rule-reminder.py`](../.claude/hooks/rule-reminder.py) parst **genau diese
Tabelle** (zwischen den `SIGNAL-TABELLE`-Markern) über **eine** gemeinsame Funktion —
derselbe Treffer speist sowohl den informativen `additionalContext` als auch das Read-Gate.
Neue Zeile hier = neuer Trigger; keine zweite Heuristik im Code.

Spaltenvertrag: `Typ` ∈ {`pfad`, `inhalt`}; `Muster` ist eine Python-Regex (in Backticks;
`pfad` matcht den repo-relativen Editpfad, `inhalt` den Datei-Inhalt **nach** dem Edit);
`Ziel` beginnt **immer** mit dem `.rules/`-relativen Regel-Pfad (Read-Gate vergleicht darauf),
optional gefolgt von `— Abschnitt`; mehrere Ziele mit `;` trennen.

<!-- SIGNAL-TABELLE:START -->

| Typ | Muster | Ziel |
|-----|--------|------|
| pfad | `crates/[^/]+_domain/` | `rust/idiome.md — „<slice>_domain ist dependency-frei"; common/domaenenmodell.md — „Value Objects statt Primitiven"` |
| pfad | `crates/[^/]+_adapter/` | `common/privacy-invarianten.md — „Kein Serialize auf Egress-DTOs"; crates/dependencies-und-kapselung.md — „Slice erreicht fremden Slice nur über dessen _adapter"` |
| pfad | `crates/[^/]+_specs/` | `common/testing.md — „*_specs sind reine Black-Box"` |
| pfad | `crates/[^/]+/Cargo\.toml$` | `crates/dependencies-und-kapselung.md — „Nur io bindet konkrete Fremd-Crates" (harte Durchsetzung zusätzlich in enforce-dependency-direction.py)` |
| pfad | `crates/io/` | `common/architektur.md — „Externe Information hinter einem Port"` |
| pfad | `\.sql$` | `sql/schema-konventionen.md; sql/queries-und-performance.md` |
| pfad | `docs/issues/` | `common/tickets.md — „Tickets doppelt anlegen — GitHub UND docs/issues/"` |
| pfad | `\bmemory/` | `common/kein-automatisches-memory.md — „Nichts selbsttätig ins Memory schreiben"` |
| inhalt | `#\[derive\([^)]*\bSerialize\b` | `common/privacy-invarianten.md — „Kein Serialize auf Egress-DTOs"` |
| inhalt | `Arc\s*<\s*Mutex\b` | `rust/nebenlaeufigkeit.md — „Message-Passing statt geteiltem Mutex"` |
| inhalt | `\basync\s+fn\b` | `rust/nebenlaeufigkeit.md — „Tokio als Async-Runtime"` |
| inhalt | `#\[tokio::` | `rust/nebenlaeufigkeit.md — „Tokio als Async-Runtime"` |
| inhalt | `SELECT\s+\*` | `sql/queries-und-performance.md — „Nur benötigte Spalten selektieren — nie SELECT *"` |

<!-- SIGNAL-TABELLE:END -->

Der Hook ist **fail-open** (jeder Fehler → Edit läuft durch). Trifft ein Edit ein Signal,
dessen Regel-Datei in dieser Session noch **nicht** per `Read` geöffnet wurde, **blockt** er
**sofort** (`exit 2`, auch in Turn 1) und verlangt, die Regel-Datei zuerst aufzuschlagen.
Bewusst so: die Regeln sind zwar via `@`-Import geladen, der erzwungene `Read` stellt aber
sicher, dass der Agent genau die betroffene Regel vor dem Edit ansieht (der bloße Reminder
wurde wiederholt überlesen). Ist die Regel gelesen, bleibt nur der gedrosselte informative
Reminder (`exit 0`).

Achtung Reichweite: Inhalts-Heuristiken matchen **überall** im Datei-Inhalt, auch in Prosa,
Tabellen oder Code-Beispielen. Diese README selbst enthält z. B. `SELECT *` und `async fn`
als Beispiel — ein Edit hier blockt daher, bis `sql/queries-und-performance.md` und
`rust/nebenlaeufigkeit.md` gelesen sind. Das ist gewollt (lieber ein Read zu viel).
