---
id: "0074"
title: StoreSpec-Abstraktion einführen und einlesen migrieren
status: closed
---

# StoreSpec-Abstraktion einführen und einlesen migrieren

Umsetzt [ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) — Kandidat 1,
erster Slice (Tracer Bullet). **AFK.**

## What to build

Die generische, `io`-interne Store-Abstraktion einführen und den **einfachsten** Slice
(`einlesen` — eine Tabelle, ID-rückgebend) darauf umstellen, sodass die Abstraktion
end-to-end an einem realen Slice bewiesen ist.

Die Abstraktion (Details und Begründung in ADR-0015, Abschnitt **Entscheidung**):

- Ein neues Trait **`StoreSpec`** mit den assoziierten Typen `Dto`/`Prepared`/`Error` und den
  Bausteinen `tables`/`schema`/`inserts`/`flatten`/`stage`/`unavailable`. `flatten` ist
  **fehlbar**, läuft **vor** dem Minten und liefert die **id-lose** `Prepared`; `inserts` listet
  **alle** Statements (für den Preflight-`prepare`). `stage(dto, prepared, id)` baut die
  id-tragende Row **in einem Zug mit der ID** (nie eine Row ohne ID); die SQL-fertige Row bleibt
  die private Struct des Slice (`LineRow`), kein assoziierter Typ.
- Die generischen Typen **`SliceStore<S>`** (trägt Struct/`new`/`UnitFactory` **einmal**)
  und **`SliceUnit<S>`** mit der generischen Ablage-Primitive `store_dto`, die den
  Seam-Body-Ablauf trägt: `flatten` → Preflight aller `inserts` → `mint_id` → `stage` (Row mit
  der frisch geminteten ID), Rückgabe **stets** der DB-geminteten ID.
- Der Outbox-Passthrough (`OutboxPublisher`/`UnitOfWork`) als **ein** generischer `impl`
  über `SliceUnit<S>` (statt pro Store per Makro).

Der `einlesen`-Store liefert nur noch sein `StoreSpec`-`impl` (Schema + Insert + Flatten)
plus eine dünne Seam-`impl`, die `store_dto` aufruft. Der Seam-Vertrag Richtung Slice
(`StoreSeam`) bleibt **unverändert** (ADR-0011). Das `sqlite_backed_unit!`-Makro **bleibt**
in diesem Slice bestehen — die drei noch nicht migrierten Stores nutzen es weiter.

## Acceptance criteria

- [ ] Die generische Store-Abstraktion (`StoreSpec` + `SliceStore`/`SliceUnit` +
      generischer Outbox-Passthrough) existiert als eigenes `io`-internes Modul.
- [ ] `einlesen` ist auf die Abstraktion migriert; sein Store trägt nur noch das
      slice-spezifische `StoreSpec`-`impl` + die dünne Seam-`impl`.
- [ ] Der `StoreSeam`-Vertrag des `einlesen`-Adapters ist unverändert (keine
      Signatur-/DTO-/Rückgabe-Änderung Richtung Slice).
- [ ] Die bestehenden `einlesen`-Store-Tests bleiben unverändert grün: feld-treue Ablage,
      terminal-redigierter Fehlschlag ohne Publish, Batch-Atomarität, ISO-4217-Skalierung.
- [ ] Alle sechs Gates grün (`/cargo-six-gates`).

## Blocked by

None - can start immediately.
