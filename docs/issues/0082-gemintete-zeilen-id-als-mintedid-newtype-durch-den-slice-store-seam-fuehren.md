---
id: "0082"
title: Gemintete Zeilen-ID als MintedId-Newtype durch den Slice-Store-Seam führen
status: closed
---

# Gemintete Zeilen-ID als MintedId-Newtype durch den Slice-Store-Seam führen

Härtet [ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) nach dem Capstone
gegen einen darstellbaren Ungültig-Zustand ([ADR-0005](../adr/0005-domaenenmodell-value-objects-und-summentypen.md),
[ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)). **AFK.**

Herkunft: Finding des `illegal-state-check` (Verdict BLOCK, Kind „Unconstrained primitive")
aus dem Validator-Lauf über die `fenstern`-Migration
([0076](0076-fenstern-migrieren-zwei-tabellen-fall.md)). Dort bewusst **vertagt**, weil es
den geteilten Seam und damit alle vier Slice-Stores anfasst.

## What to build

Die an der DB gemintete Zeilen-ID reist heute als nackter `String` durch den geteilten
Slice-Store-Seam: die generische Ablage-Primitive mintet sie, reicht sie an
`StoreSpec::stage` weiter, und der Slice-Store baut daraus seine id-tragende Row. Eine leere
oder frei erfundene ID ist damit **konstruierbar** — sie schriebe Kopf- und Detail-Zeilen
unter einem Schein-Primärschlüssel und reiste als publizierter Zeiger stromabwärts. Die
Doc-Kommentare räumen das wörtlich ein („Konvention an einer Stelle, keine Typ-Garantie");
einen Check gibt es nicht einmal.

Ein `MintedId`-Newtype in der io-internen SQLite-Schicht macht den Zustand
unrepräsentierbar statt bloß unwahrscheinlich: konstruierbar **ausschließlich** dort, wo die
ID an der DB gemintet wird (privates Feld, kein öffentlicher Konstruktor), durchgereicht
durch `StoreSpec::stage` und die Rückgabe der Ablage-Primitive. Alle vier Slice-Stores
(`einlesen`, `obfuskierung`, `fenstern`, `auswerten`) ziehen mit. Der Seam-Vertrag Richtung
Slice-Adapter bleibt **unberührt** ([ADR-0011](../adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
die Slices sehen weiterhin ihre `String`-ID und publizieren sie als Zeiger. Reine
Typ-Härtung, kein Verhaltenswechsel — die abgelegten Zeilen und die publizierten Zeiger
bleiben identisch.

## Acceptance criteria

- [ ] `MintedId` existiert io-intern und ist außerhalb des Mint-Pfads **nicht**
      konstruierbar (privates Feld, kein öffentlicher Konstruktor) — eine von Hand gebaute
      Schein-ID ist ein **Compile-Fehler**, keine Review-Frage.
- [ ] Die generische Ablage-Primitive reicht `MintedId` an `StoreSpec::stage` durch; alle
      vier Slice-Stores bauen ihre id-tragende Row daraus.
- [ ] Der Seam-Vertrag Richtung Slice bleibt unverändert: die Stores geben weiterhin die
      `String`-ID zurück, die als Zeiger publiziert wird (das Pipeline-Ende verwirft sie
      weiterhin per `map(drop)`).
- [ ] Die Doc-Kommentare, die die fehlende Typ-Garantie einräumen, sind durch die nun
      geltende Zusage ersetzt — an **allen** Stellen, die sie heute tragen.
- [ ] Die bestehenden Store-Tests aller vier Slices bleiben unverändert grün.
- [ ] Alle sechs Gates grün (`/cargo-six-gates`).

## Blocked by

- [0077](0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md) —
  erst wenn **alle** vier Slice-Stores auf der generischen Abstraktion sitzen, ist die
  Seam-Signatur an *einer* Stelle änderbar statt an vier Baustellen gleichzeitig.
