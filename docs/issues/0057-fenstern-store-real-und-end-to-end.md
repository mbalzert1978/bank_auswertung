---
id: "0057"
title: "fenstern-Store real + Lese-Seams + fenstern end-to-end real (Tracer-Bullet)"
status: open
---

# fenstern-Store real + Lese-Seams + fenstern end-to-end real (Tracer-Bullet)

**Typ:** AFK

> GitHub-Issue: [#57](https://github.com/mbalzert1978/bank_auswertung/issues/57)

## What to build

Der reale `fenstern`-Ergebnis-Store (rusqlite, erfüllt den `AnalysisWindowStoreSeam` + die
`UnitFactory` des Slice) **und** die realen Lese-Seams über die `obfuskiert`-Tabelle — die **fertige
Tracer-Bullet**: der `fenstern`-Slice pagt end-to-end **ohne Fake**, Replikation des Musters aus
[0056](0056-obfuskierung-store-real-und-end-to-end.md).

Real werden:

- der **`WindowAnchor`**-Port: der Anstoß (`StartAnalysis`) liest den ersten Anker `max(booking_day)`
  über echtes SQL; der Fenster-Schritt den nächsten Anker `max(booking_day < from)` — datenrelativ,
  keine Uhr; **leerer Bestand → `Ok`, publiziert nichts**,
- der **`ObfuscatedWindowSeam`**: liest die reale Seite `[anchor − window_days + 1 … anchor]` aus der
  `obfuskiert`-Tabelle (message-unabhängig, außerhalb der committenden Transaktion),
- der **`AnalysisWindowStoreSeam`**: schreibt **ein** `Auswertungsfenster` je Seite in die echte
  Fenster-Tabelle (DB-vergebene ID; Invarianten bei Konstruktion: nicht-leer + eine Währung).

Das **Committende** — das abgelegte Fenster, der `FensterGeschlossen`-Zeiger (neue ID) **und** das
nächste `AdvanceWindow` bzw. das terminale `HistorieVollständig` — liegt in **einer** Transaktion:
so entsteht nie eine halb-fortgesetzte Pager-Kette. Read/Store/Publish je **fehlbar + terminal +
redigiert** ([ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)). Die
Seitengröße `WindowDays` kommt schon real via [0054](0054-config-loader-toml-fenster-config.md) als Wert.

## Acceptance criteria

- [ ] `AnalysisWindowStoreSeam` durch eine echte rusqlite-Fenster-Tabelle erfüllt (DB-ID; Aggregat-Invarianten
      nicht-leer + eine Währung bei Konstruktion).
- [ ] `ObfuscatedWindowSeam` liest die reale Seite aus der `obfuskiert`-Tabelle; `WindowAnchor` liefert
      `max(booking_day)` bzw. `max(booking_day < from)` über echtes SQL; Lesevorgänge außerhalb der Transaktion.
- [ ] Ein End-to-End-Test über **mehrere Seiten**: Anstoß → Fenster materialisieren → Selbst-Fortsetzung
      (Lücken übersprungen, Fenster nie leer) → `HistorieVollständig` bei Erschöpfung; leerer Bestand → `Ok`,
      publiziert nichts. Fenster-Ablage + alle ausgehenden Nachrichten atomar in **einer** Transaktion.
- [ ] Ein terminaler Fehler auf **jedem** I/O-Schritt failt terminal + redigiert; kein Dead-Letter/Retry.
- [ ] Im fenstern-io-Pfad ist **kein Fake-Store und keine Fake-Lese-Quelle** mehr gebunden;
      `cargo test -p architecture` bleibt grün.
- [ ] Alle sechs Gates grün.

## Blocked by

- Blocked by [0056](0056-obfuskierung-store-real-und-end-to-end.md) — die reale `obfuskiert`-Tabelle ist
  die Lese-Quelle dieses Slice.
