---
id: "0072"
title: "io/Stores: leere Platzhalter-ID in `obfuskierung`/`fenstern`-Rows unkonstruierbar machen"
status: closed
---

# io/Stores: leere Platzhalter-ID in `obfuskierung`/`fenstern`-Rows unkonstruierbar machen

**Typ:** AFK

> GitHub-Issue: [#78](https://github.com/mbalzert1978/bank_auswertung/issues/78)

## What to build

Die realen slice-Stores flachen ihr Aggregat vor dem ID-Minten in eine SQL-fertige Row ab.
Zwei von ihnen konstruieren diese Row mit einem **illegalen leeren Platzhalter**
`id: String::new()` und setzen die echte, DB-gemintete ID erst danach per `with_id`:

- `obfuskierung`: `BookingRow` (`crates/io/src/obfuskierung/store.rs`, `id: String::new()` + `with_id`)
- `fenstern`: `WindowRow` (`crates/io/src/fenstern/store.rs:128` + `with_id`, `fenstern/store.rs:139`)

Damit existiert zwischen `try_from` und `with_id` ein halb-gebautes Row-Objekt mit unzulässiger
leerer ID — eine Invariante, die **zwischen zwei Schritten verletzbar** ist (Anti-Muster laut
Domänenmodell-Regel: illegale Zustände unkonstruierbar machen). Kein Live-Bug (`save` ruft immer
`with_id` vor dem Stagen), aber ein latenter Modell-Geruch.

`einlesen` macht es bereits richtig: `LineRow::new(id, line, amount_minor_units)` nimmt die ID
**am Konstruktor** (`crates/io/src/einlesen/store.rs`).

**Ziel:** den leeren Platzhalter unkonstruierbar machen — `obfuskierung` und `fenstern` an
`einlesen`s Form angleichen. Die abgeflachte Form **ohne** ID modellieren (z. B.
`PendingBookingRow`/`PendingWindowRow`) und die ID erst beim Übergang in die schreibbare Row
hereingeben, z. B. `fn into_insert(self, id: String) -> …`. Nach dem Fix trägt keine
konstruierbare Row je eine leere ID.

Reines Modell-Tightening: **kein** Verhaltens-, Schema- oder Message-Wechsel. Redigiert bleibt
Pflicht (keine rohen Identifikatoren in Commit/PR).

## Acceptance criteria

- [ ] Keine slice-Store-Row lässt sich mit leerer/Platzhalter-ID konstruieren; die ID kommt am Übergang in die schreibbare Form herein (wie `einlesen`s `LineRow::new`).
- [ ] Das `with_id(String::new())`-Platzhaltermuster in `obfuskierung`/`fenstern` ist entfernt.
- [ ] Kein geändertes Testverhalten, kein Schema-/Message-Wechsel.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- None — unabhängig. Berührt dieselben Store-Dateien wie das Test-Support-Extraktions-Ticket, aber einen anderen Code-Bereich (Produktions-Rows vs. Testmodule).
