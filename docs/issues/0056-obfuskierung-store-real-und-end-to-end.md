---
id: "0056"
title: "obfuskierung-Store real + Lese-Seam + obfuskierung end-to-end real (Tracer-Bullet)"
status: open
---

# obfuskierung-Store real + Lese-Seam + obfuskierung end-to-end real (Tracer-Bullet)

**Typ:** AFK

> GitHub-Issue: [#55](https://github.com/mbalzert1978/bank_auswertung/issues/55)

## What to build

Der reale `obfuskierung`-Ergebnis-Store (rusqlite, erfüllt den `ObfuscatedStoreSeam` + die
`UnitFactory` des Slice) **und** die reale Lese-Quelle (`IngestedSourceSeam`, liest die
`einlesen`-Zeile per ID) — die **fertige Tracer-Bullet**: der `obfuskierung`-Slice läuft end-to-end
**ohne einen einzigen Fake**. Es ist die **Replikation des einlesen-Musters**
([0053](0053-einlesen-store-real-und-end-to-end.md)) über die Datenschutz-Grenze.

Ablauf, das **Committende** in **einer** Transaktion:

1. eine `Ingested`-Nachricht triggert; ihr `partition_key` ist die einlesen-ID (Zeiger),
2. die reale `einlesen`-Zeile wird per ID über den `IngestedSourceSeam` gelesen (message-unabhängig,
   idempotent, **außerhalb** der Transaktion — die einlesen-Zeile ist append-only),
3. seed-basiert obfuskiert (der `ObfuscatorSeam`; den Seed reicht der Aufrufer als Wert herein),
4. das obfuskierte Aggregat in die **echte `obfuskiert`-Tabelle** geschrieben (die Persistenz
   vergibt die neue ID; referenziert die Quell-Buchung **nur per ID**),
5. **die neue ID** als `Obfuscated`-Nachricht über den echten Broker
   ([0052](0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md)) publiziert — Ablage + Nachricht
   + Ack committen gemeinsam oder gar nicht.

Read, Store und Publish sind je **fehlbar + terminal**; ein terminaler Fehler (fehlendes
Pflichtfeld, unlesbare/fehlende Quell-Zeile, Store-/Publish-Fehler) surfacet als **nackter,
redigierter** Domänen-Fehler (statischer Feldname / opake ID, nie ein Rohwert) — **kein**
Dead-Letter, **kein** Retry ([ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)).

Das ist das **Muster für 0057/0058** (je realer Store hinter dem eigenen Seam + reale Lese-Seams
über die Vorgänger-Tabelle).

## Acceptance criteria

- [ ] Der `ObfuscatedStoreSeam` ist durch eine echte rusqlite-`obfuskiert`-Tabelle erfüllt (DB-vergebene
      neue ID; das Aggregat referenziert die Quell-Buchung **nur per ID**).
- [ ] Der `IngestedSourceSeam` liest die reale `einlesen`-Zeile per ID (kein Fake mehr); der Lesevorgang
      bleibt außerhalb der committenden Transaktion.
- [ ] Ein End-to-End-Test: reale einlesen-Zeile → seed-basierte Obfuskierung → reale `obfuskiert`-Tabelle
      + reale `Obfuscated`-Outbox-Nachricht (die **neue** ID als `partition_key`), atomar in **einer**
      Transaktion.
- [ ] Ein terminaler Fehler auf **jedem** der drei I/O-Schritte failt terminal + redigiert (nie ein
      Rohwert / keine Roh-ID im Fehler); kein Dead-Letter/Retry.
- [ ] Im obfuskierung-io-Pfad ist **kein Fake-Store und keine Fake-Quelle** mehr gebunden (der Seed bleibt
      ein hereingereichter Wert); `cargo test -p architecture` bleibt grün.
- [ ] Alle sechs Gates grün: `build`, `test`, `clippy --all-targets`, `fmt --check`,
      `cargo test -p architecture`, `cargo doc --no-deps --workspace`.

## Blocked by

- None — kann sofort starten. Die reale `einlesen`-Tabelle ([0053](0053-einlesen-store-real-und-end-to-end.md))
  und das transaktionale Rückgrat ([0052](0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md))
  stehen bereits in `main`.
