---
id: "0069"
title: "Gegenprüfung `io` gegen die `.rules`"
status: open
---

# Gegenprüfung `io` gegen die `.rules`

**Typ:** AFK

> GitHub-Issue: [#68](https://github.com/mbalzert1978/bank_auswertung/issues/68)

## What to build

Gegenprüfung des Codes von `crates/io` gegen die nach `.rules/` ausgelagerten Regeln.
**Umsetzungs-Ticket** — der Code von `crates/io` wird gegen jede unten gelistete einschlägige
`.rules`-Regel geprüft und, **wo eine Regel verletzt ist, im Code angepasst**, bis die Crate
ihren `.rules` entspricht. Ist die Crate bereits konform, bleibt sie unverändert — **keine
künstliche Änderung, kein neues Ticket, kein Audit-Report**. Redigiert bleibt Pflicht: keine
rohen Identifikatoren/Klartexte in Commit/PR. `io` ist der einzige Ort mit Fremd-Crates, SQL
und Nebenläufigkeit, daher greifen hier die meisten Regeln.

Einschlägige Regeln:

- `.rules/crates/dependencies-und-kapselung.md` — **nur `io` bindet konkrete Fremd-Crates**
  (`rusqlite`/`csv`/`toml`): positiv prüfen, dass alle I/O-Crates hier und **nirgends sonst**
  liegen.
- `.rules/sql/schema-konventionen.md` — snake_case + Plural-Tabellen;
  `created_at`/`updated_at`; Check-Constraints für enum-artige Werte; PK-Wahl (Hinweis: reale
  Persistenz ist **SQLite** — Postgres-Spezifika sinngemäß, ADR-0014).
- `.rules/sql/queries-und-performance.md` — nie `SELECT *`; teure Aggregationen vorberechnen
  (SQLite-Äquivalent: Ergebnistabelle mit Neuaufbau); FK-Spalten indizieren.
- `.rules/sql/migrationen.md` — Migrationen auf Datenkopie testen; `NOT NULL` mit `DEFAULT`.
- `.rules/common/architektur.md` — `io` klammert lesen/ablegen/publizieren in **eine**
  Transaktion (Outbox, `UnitOfWork`); erfüllt die Ports (`Calendar`/`Money`/Source/Store);
  durchweg terminal (io surfacet den Fehlschlag); uniformes ID-Handoff (Persistenz vergibt die ID).
- `.rules/rust/nebenlaeufigkeit.md` — Tokio als Runtime / Message-Passing; Abgleich mit
  **ADR-0014** (Outbox-Spine bleibt **synchron**) — prüfen, dass keine zweite Runtime / kein
  ungewollter Async-Ripple existiert.
- `.rules/rust/testing-werkzeuge.md` — **`cargo-fuzz`** für den sicherheitskritischen
  CSV-CAMT-v8-Parser (vorhanden / fehlend notieren).
- `.rules/common/privacy-invarianten.md` — rohe IBAN nur lokal bis `obfuskieren`; der Seed
  bleibt lokal (io verdrahtet ihn, er erreicht kein Egress-DTO).
- `.rules/rust/performance.md` — Zero-Cost-Iteratoren; `[profile.release]` LTO +
  `codegen-units = 1` (Workspace-`Cargo.toml`).
- `.rules/rust/idiome.md` (Newtypes; hier gilt die Fremd-Crate-Ausnahme),
  `.rules/common/sprache.md`, `.rules/rust/gates.md`.

## Acceptance criteria

- [ ] Jede gelistete Regel ist erfüllt; wo sie verletzt war, wurde der Code an die Regel angepasst.
- [ ] SQL-Schema/Queries/Migrationen der real gelandeten Adapter (Outbox-Broker, `einlesen`-Store,
      Calendar/Money, CSV-Source, `toml`-Loader) entsprechen den drei SQL-Regeldateien; wo verletzt, angepasst.
- [ ] Alle Fremd-Crates ausschließlich in `io`; keine zweite Async-Runtime (Spine synchron, ADR-0014) — sonst hergestellt.
- [ ] Fuzz-Ziel für den CSV-CAMT-v8-Parser vorhanden; fehlte es, ergänzt (oder bewusst vertagt begründet); Release-Profil (LTO/codegen-units) regelkonform, sonst gesetzt.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).
- [ ] PR nur bei echter Code-Anpassung; ist die Crate bereits konform, keine Änderung und kein PR.

## Blocked by

- None — kann sofort starten (unabhängiges Umsetzungs-Ticket).
