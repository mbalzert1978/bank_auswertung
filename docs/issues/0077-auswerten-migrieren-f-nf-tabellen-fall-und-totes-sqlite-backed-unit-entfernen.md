---
id: "0077"
title: auswerten migrieren (Fünf-Tabellen-Fall) und totes sqlite_backed_unit entfernen
status: blocked
---

# auswerten migrieren (Fünf-Tabellen-Fall) und totes sqlite_backed_unit entfernen

Umsetzt [ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) — Kandidat 1,
vierter und letzter Slice (Capstone). **AFK.**

## What to build

Den `auswerten`-Store auf die generische `StoreSpec`-Abstraktion umstellen — der
komplexeste Fall — und als **letzte** Migration das dann tote Makro entfernen.

- `auswerten` schreibt **fünf** Tabellen (Kopf + vier positionsgetreue Detail-Listen). Das
  `StoreSpec`-`impl` deklariert alle fünf über `tables`/`schema`, führt **alle**
  Insert-Statements in `inserts` (Preflight probt jedes) und baut in `stage` eine Closure,
  die Kopf- und alle Detail-Inserts positionsgetreu schreibt.
- Die fehlbare u64→i64-Vor-Umdeutung der Monats-Beträge liegt im **`flatten`** (vor dem
  Minten). Diese Reichweite ist das **Pipeline-Ende** und publiziert nichts: die
  Seam-`impl` verwirft die von `store_dto` gemintete ID via `map(drop)` und gibt `()`
  zurück (die ID bleibt DB-intern vergeben, ADR-0012). Der Seam-Vertrag
  (`EvaluationStoreSeam`) bleibt unverändert.
- Nach dieser Migration nutzt **kein** Store mehr das `sqlite_backed_unit!`-Makro. Die
  Makro-Definition **und** ihr `pub(crate) use`-Re-Export werden entfernt, damit der
  `-Dwarnings`-Lauf nicht an totem Code/ungenutztem Import scheitert.

## Acceptance criteria

- [ ] `auswerten` ist auf die Abstraktion migriert; alle fünf Tabellen laufen über **ein**
      `StoreSpec`-`impl`.
- [ ] Die `()`-Rückgabe des Pipeline-Endes ist über `map(drop)` der geminteten ID gelöst
      (kein zweiter Seam, kein `Option`); die ID bleibt DB-vergeben.
- [ ] Der Preflight probt **alle** Insert-Statements (Kopf + vier Detail) — ein fehlendes
      Detail-Schema failt terminal beim `save`.
- [ ] Das `sqlite_backed_unit!`-Makro und sein Re-Export sind entfernt; kein Store
      referenziert es mehr.
- [ ] Die bestehenden `auswerten`-Store-Tests bleiben unverändert grün.
- [ ] Alle sechs Gates grün (`/cargo-six-gates`) — insbesondere `clippy`/`-Dwarnings` ohne
      Toter-Code-/Ungenutzt-Warnung.

## Blocked by

- [0074](0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md) — führt die
  `StoreSpec`-Abstraktion ein.
- [0075](0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md) — muss
  migriert sein, damit das Makro tot ist.
- [0076](0076-fenstern-migrieren-zwei-tabellen-fall.md) — muss migriert sein, damit das
  Makro tot ist.
