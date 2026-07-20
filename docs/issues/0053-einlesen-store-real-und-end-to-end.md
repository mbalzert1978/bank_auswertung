---
id: "0053"
title: "einlesen-Store real + einlesen end-to-end real (Tracer-Bullet)"
status: open
---

# einlesen-Store real + einlesen end-to-end real (Tracer-Bullet)

**Typ:** AFK

## What to build

Der reale einlesen-Ergebnis-Store (rusqlite, erfüllt den `StoreSeam` / Domänen-`Store`) **und** die
volle Real-Verdrahtung des einlesen-Slice — die **fertige Tracer-Bullet**: der erste Slice läuft
end-to-end **ohne einen einzigen Fake**.

Ablauf, alles in **einer** Transaktion:

1. eine Fixture-CSV wird über die echte Quelle ([0051](0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md)) gelesen,
2. über die echten `Money`/`Calendar`-Seams ([0050](0050-money-calendar-decode-seams-real.md)) dekodiert,
3. das `LineAggregate` gebaut, in die **echte einlesen-Tabelle** geschrieben (die Persistenz vergibt die ID),
4. **eine `Ingested`-Nachricht pro gültiger Zeile** über den echten Broker
   ([0052](0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md)) publiziert.

Read, Store und Publish sind je **fehlbar + terminal**; ein terminaler Fehler (Quelle / Präsenz /
Decode / Store / Publish) surfacet als **nackter, redigierter** Domänen-Fehler — **kein**
Dead-Letter, **kein** Retry ([ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)).

Das ist das **Muster für die Replikation** über obfuskieren/fenstern/auswerten (je realer Store hinter
dem eigenen Seam) — die werden **nach** diesem Ticket geschnitten, wenn das Muster steht.

## Acceptance criteria

- [ ] Der `StoreSeam` ist durch eine echte rusqlite-einlesen-Tabelle erfüllt (DB-vergebene ID; Aggregat
      referenziert nichts Fremdes außer per ID).
- [ ] Ein End-to-End-Test: Fixture-CSV → echte Dekodierung → echte einlesen-Tabelle + echte
      `Ingested`-Outbox-Nachrichten (**eine je gültiger Zeile**), atomar in **einer** Transaktion.
- [ ] Ein terminaler Fehler auf **jedem** der drei I/O-Schritte failt den Batch terminal + redigiert
      (nie ein Rohwert im Fehler); kein Dead-Letter/Retry.
- [ ] Im einlesen-io-Pfad ist **kein Fake mehr** gebunden; `cargo test -p architecture` bleibt grün.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
      `cargo test -p architecture` und `cargo doc --no-deps --workspace` sind grün.

## Blocked by

- Blocked by [0050](0050-money-calendar-decode-seams-real.md) — echte Dekodierung ist Teil der Kette.
- Blocked by [0051](0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md) — echte CSV-Quelle ist
  der Kopf der Kette.
- Blocked by [0052](0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md) — Store + Publish
  brauchen die transaktionale Backbone.
