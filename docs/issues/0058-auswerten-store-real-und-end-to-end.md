---
id: "0058"
title: "auswerten-Store real + Lese-Seams + volle Pipeline end-to-end real (Tracer-Bullet)"
status: closed
---

# auswerten-Store real + Lese-Seams + volle Pipeline end-to-end real (Tracer-Bullet)

**Typ:** AFK

> GitHub-Issue: [#58](https://github.com/mbalzert1978/bank_auswertung/issues/58)

## What to build

Der reale `auswerten`-Ergebnis-Store (rusqlite, erfüllt den `EvaluationStoreSeam` + die `UnitFactory`
des Slice) **und** die realen Lese-Seams über die Fenster-Tabelle — die **letzte** Store-Tracer-Bullet:
danach läuft die **volle Pipeline** `einlesen → obfuskieren → fenstern → auswerten` end-to-end **ohne
Fake** (bis zum `report`).

Real werden die **beiden Reichweiten** (ADR-0013, Entscheidung E):

- **pro-Fenster** (`FensterGeschlossen`, ID-Zeiger): der `AnalysisWindowSeam` liest das gefrorene
  `Auswertungsfenster` per ID aus der realen Fenster-Tabelle; das Ergebnis (heute Monats-Cashflow +
  distinct Fingerabdrücke) wird über den `EvaluationStoreSeam` in die echte Ergebnis-Tabelle abgelegt,
- **fenster-übergreifend** (`HistorieVollständig`, run-scoped): der `OccurrenceSeam` läuft **eine**
  Read-Port-Query über **alle** Fenster des Laufs und **gibt** die Vorkommen zurück — **legt nichts ab**
  (Idempotenz aus der Abfrage unveränderlicher Snapshots, kein Fold-Log).

Read/Store/Query je **fehlbar + terminal + redigiert**; **kein Publish-Seam** (Pipeline-Ende bis zum
`report`) — die Unit umspannt Ack + Ablage, der Publish-Rand bleibt ungenutzt, wächst aber ohne Umbau
für den späteren `report`-Anstoß hinein.

**Abgrenzung (kein Blocker dieses Tickets):** die token-orientierte **Eigene-Konten-Config**
([0002](0002-eigene-konten-config-schema-und-loader.md)) und der **Kategorisierer**-Adapter
([0011](0011-kategorisierer-chat-payload-adapter.md)) bleiben separate Tickets — der e2e-Test reicht den
Eigene-Konten-Wert als Wert herein (wie heute, ADR-0013 Entscheidung F) und nutzt einen Test-Doppel für
das Orakel. Dieses Ticket ersetzt weder 0002 noch 0011.

## Acceptance criteria

- [ ] `EvaluationStoreSeam` durch eine echte rusqlite-Ergebnis-Tabelle erfüllt (pro-Fenster-Reichweite legt ab).
- [ ] `AnalysisWindowSeam` liest das `Auswertungsfenster` per ID aus der realen Fenster-Tabelle;
      `OccurrenceSeam` läuft **eine** Query über **alle** Fenster des Laufs und legt nichts ab.
- [ ] Ein End-to-End-Test der **vollen Pipeline** gegen echte Persistenz: Fixture-CSV → … → reale
      Ergebnis-Tabelle; beide Reichweiten real (pro-Fenster ablegend, fenster-übergreifend nur lesend).
- [ ] Ein terminaler Fehler auf **jedem** I/O-Schritt (Read/Store/Query) failt terminal + redigiert; kein
      Dead-Letter/Retry.
- [ ] Im auswerten-io-Pfad ist **kein Fake-Store und keine Fake-Lese-Quelle** mehr gebunden (Eigene-Konten
      + Orakel bleiben hereingereichte Werte/Test-Doppel); `cargo test -p architecture` bleibt grün.
- [ ] Alle sechs Gates grün.

## Blocked by

- Blocked by [0057](0057-fenstern-store-real-und-end-to-end.md) — die reale Fenster-Tabelle ist die
  Lese-Quelle dieses Slice.
