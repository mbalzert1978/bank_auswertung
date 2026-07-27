---
id: "0075"
title: obfuskierung migrieren und StoreUnavailable vereinheitlichen
status: closed
---

# obfuskierung migrieren und StoreUnavailable vereinheitlichen

Umsetzt [ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) — Kandidat 1,
zweiter Slice. **AFK.**

## What to build

Den `obfuskierung`-Store auf die generische `StoreSpec`-Abstraktion (aus dem Blocker)
umstellen und dabei die **Fehler-Form-Drift** auflösen, die ADR-0015 dokumentiert.

- `obfuskierung` bekommt sein `StoreSpec`-`impl` (Schema + Insert + Flatten) und eine dünne
  Seam-`impl`, die `store_dto` aufruft. Der Store ist ID-rückgebend (publiziert den
  `Obfuscated`-Zeiger) — der Seam-Vertrag (`ObfuscatedStoreSeam`) bleibt unverändert.
- Die **`StoreUnavailable`-Form** wird vereinheitlicht: der `obfuskierung`-Adapter-Fehler
  ist heute als einziger **feldlos**, die drei anderen tragen eine redigierte `message`.
  Künftig trägt auch `obfuskierung` eine redigierte `message` (ausschließlich
  io-Zugriffs-Mechanik, nie ein DTO-Feld- oder Zellwert — Privacy-Invariante, ADR-0015
  Punkt 5). Die Black-Box-`_specs` des Slice, die den Fehler heute feldlos konstruieren,
  ziehen den geänderten Konstruktor mit.

## Acceptance criteria

- [x] `obfuskierung` ist auf die Abstraktion migriert; der Store trägt nur noch sein
      `StoreSpec`-`impl` + die dünne Seam-`impl`.
- [x] Der `obfuskierung`-Adapter-Fehler `StoreUnavailable` trägt eine redigierte
      Text-Diagnose, formgleich mit den drei anderen Slices.
- [x] Die redigierte Diagnose enthält **keinen** Roh-Feld-/Zellwert (Privacy-Invariante,
      analog zum bestehenden `einlesen`-Leak-Test).
- [x] Die bestehenden `obfuskierung`-Store- und `_specs`-Tests bleiben grün (Konstruktor
      der geänderten Fehler-Form nachgezogen).
- [x] Alle sechs Gates grün (`/cargo-six-gates`).

## Blocked by

- [0074](0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md) — führt die
  `StoreSpec`-Abstraktion ein.
