---
id: "0076"
title: fenstern migrieren (Zwei-Tabellen-Fall)
status: closed
---

# fenstern migrieren (Zwei-Tabellen-Fall)

Umsetzt [ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) — Kandidat 1,
dritter Slice. **AFK.**

## What to build

Den `fenstern`-Store auf die generische `StoreSpec`-Abstraktion umstellen — der erste
**Mehr-Tabellen-Fall** und damit der Beweis, dass die Abstraktion Kopf + Detail trägt.

- `fenstern` schreibt **zwei** Tabellen (das Fenster und seine Buchungs-Fremdschlüssel).
  Das `StoreSpec`-`impl` deklariert beide über `tables`/`schema`, führt beide Statements in
  `inserts` (sodass der Preflight beide probt) und baut in `stage` **eine** Closure, die
  Kopf- und Detail-Inserts positionsgetreu (`enumerate`) schreibt.
- Der bisherige `numbered_bookings`-Positionsvorlauf wandert in das **fehlbare `flatten`**
  (läuft vor dem Minten). Der Store ist ID-rückgebend (`FensterGeschlossen`-Zeiger); der
  Seam-Vertrag (`AnalysisWindowStoreSeam`) bleibt unverändert.

## Acceptance criteria

- [x] `fenstern` ist auf die Abstraktion migriert; beide Tabellen laufen über **ein**
      `StoreSpec`-`impl`.
- [x] Der Preflight probt **beide** Insert-Statements (Kopf + Detail) — ein fehlendes
      Detail-Schema failt terminal beim `save`, nicht erst als Commit-Retry.
- [x] Kopf- und Detail-Zeilen werden positionsgetreu in **einer** committenden Transaktion
      geschrieben (Atomarität unverändert).
- [x] Die bestehenden `fenstern`-Store-Tests bleiben unverändert grün.
- [x] Alle sechs Gates grün (`/cargo-six-gates`).

## Blocked by

- [0074](0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md) — führt die
  `StoreSpec`-Abstraktion ein.
