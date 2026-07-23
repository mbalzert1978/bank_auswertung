---
id: "0073"
title: "`fenstern`-Store: Preflight in `save` probt nur `INSERT_WINDOW`, nicht `INSERT_BOOKING`"
status: open
---

# `fenstern`-Store: Preflight in `save` probt nur `INSERT_WINDOW`, nicht `INSERT_BOOKING`

**Typ:** AFK

> GitHub-Issue: [#79](https://github.com/mbalzert1978/bank_auswertung/issues/79)

## What to build

`FensternUnit::save` sichert die Store-Erreichbarkeit über
`crate::sqlite::mint_id(conn, INSERT_WINDOW)` (`crates/io/src/fenstern/store.rs:80`). Der
Preflight prüft nur die Präparierbarkeit von `INSERT_WINDOW` (Tabelle `auswertungsfenster`),
**nicht** von `INSERT_BOOKING` (Tabelle `auswertungsfenster_buchungen`).

Existiert die Fenster-Tabelle, fehlt aber die Buchungs-Tabelle, geht der Preflight durch, das
Fenster-`INSERT` wird gestaget, und das Buchungs-`INSERT` scheitert erst **am Commit** → es
surfacet als **retrybarer** `SqliteCommitError`/`OutboxError::Retry` statt als terminaler
`StoreUnavailable`. Genau das soll der Preflight verhindern (terminal in `save`, nicht als
Commit-Retry).

Praktisch unerreichbar, weil beide Tabellen gemeinsam in `SCHEMA`
(`fenstern/store.rs:164-180`) via `new()` entstehen — daher **niedrige Priorität**. Aber die
Preflight-Zusage gilt heute nur halb.

**Ziel (eine der beiden Varianten wählen und umsetzen):**

- (a) In `save` **beide** Statements proben (`INSERT_WINDOW` **und** `INSERT_BOOKING`) vor dem Minten; oder
- (b) die Rest-Lücke bewusst akzeptieren und im Doc-Kommentar von `save` (bzw. `mint_id`) ausdrücklich benennen, warum sie unerreichbar ist.

## Acceptance criteria

- [ ] Entweder proben beide Insert-Statements terminal in `save`, oder die Rest-Lücke ist im Doc-Kommentar ausdrücklich als bewusst/unerreichbar dokumentiert.
- [ ] Kein Verhaltenswechsel für den erreichbaren Normalpfad.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- None.
