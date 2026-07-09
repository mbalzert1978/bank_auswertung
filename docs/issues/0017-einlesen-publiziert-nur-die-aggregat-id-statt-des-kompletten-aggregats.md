---
id: "0017"
title: einlesen publiziert nur die Aggregat-ID statt des kompletten Aggregats
status: open
---

# einlesen publiziert nur die Aggregat-ID statt des kompletten Aggregats

**Typ:** AFK

> Neu aus [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md) /
> [ADR-0009](../adr/0009-kernel-split-und-idiomatische-outbox-vertraege.md).
> Verschlankt die Outbox-Kopplung `einlesen → obfuskieren`: die Message wird vom
> **Nutzlast-Träger** zum **Zeiger**. Zukunfts-Aufgabe — blockiert [0016](0016-obfuskieren-slice-als-pipeline-consumer.md)
> **nicht**; 0016 konsumiert vorerst weiter die volle `Ingested`-Message.

## Was zu bauen ist

Heute fächert der `einlesen`-Slice pro gültiger Zeile eine vollständige
`Ingested`-Outbox-Message aus, die das **komplette abgeflachte Aggregat** (alle Felder)
trägt. Stattdessen soll die Message nur noch die **ID** des eingelesenen Aggregats
tragen; der nachgelagerte `obfuskieren`-Consumer liest das vollständige Aggregat dann
**per ID aus dem einlesen-Speicher** (Roh-Tabelle), statt es aus der Message zu
rekonstruieren.

Das verschiebt die Datenhoheit sauber: die **Tabelle ist die Wahrheit**, die **Message
ist der Pointer**. Rohe Felder werden nicht mehr über die Message dupliziert, die
Outbox-Zeilen bleiben schlank.

- **einlesen:** vergibt jedem eingelesenen Aggregat eine **stabile ID**, schreibt die
  Roh-Erfassung unter dieser ID und publiziert `Ingested` mit **nur der ID** (statt der
  abgeflachten Felder) — weiterhin in **einer** `UnitOfWork`.
- **einlesen-Speicher:** ein **lesbarer** Speicher-Port (die Roh-Tabelle als Read-Port),
  über den ein nachgelagerter Consumer ein Aggregat per ID holt.
- **obfuskieren:** konsumiert die schlanke `Ingested`-Message (nur ID), **liest** das
  Aggregat per ID aus dem einlesen-Speicher und obfuskiert es (Umbau des in
  [0016](0016-obfuskieren-slice-als-pipeline-consumer.md) gebauten Consume-Pfads von
  „Message trägt Nutzlast" auf „Message trägt ID, Speicher liefert Nutzlast").

## Abnahmekriterien

- [ ] Jedes eingelesene Aggregat trägt eine stabile, laufübergreifend eindeutige ID;
      die Roh-Erfassung wird unter dieser ID persistiert.
- [ ] Die `Ingested`-Outbox-Message trägt **nur** die Aggregat-ID (keine abgeflachten
      Rohfelder mehr) — über Fakes im Test belegt.
- [ ] Ein lesbarer einlesen-Speicher-Port liefert ein Aggregat per ID; `obfuskieren`
      hängt für den Lesezugriff nur über injizierte `shared_kernel`-Interfaces am
      einlesen-Speicher, keine `adapter→adapter`-Kante.
- [ ] `obfuskieren` konsumiert die schlanke Message und liest die Nutzlast aus dem
      Speicher; die Slice-Specs bleiben Black-Box über die Fluent-TestApi mit Fakes.
- [ ] Der `architecture`-Test bleibt grün; `cargo build`/`test`/`clippy --all-targets`
      und `cargo fmt --check` grün.

## Blockiert durch

- Baut auf [0015](0015-einlesen-slice-als-pipeline-consumer.md) (einlesen-Consumer +
  Roh-Erfassung) und [0016](0016-obfuskieren-slice-als-pipeline-consumer.md)
  (obfuskieren-Consume-Pfad, der hier von Nutzlast auf ID umgestellt wird) auf.
- Verwandt: [0008](0008-persistente-pseudonym-zuordnung.md) (persistente Speicher
  kommen mit dem `io`-Crate).
