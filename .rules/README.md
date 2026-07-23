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
