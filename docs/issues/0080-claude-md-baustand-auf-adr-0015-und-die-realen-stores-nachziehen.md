---
id: "0080"
title: CLAUDE.md-Baustand auf ADR-0015 und die realen Stores nachziehen
status: closed
---

# CLAUDE.md-Baustand auf ADR-0015 und die realen Stores nachziehen

Behebt eine Doku-Drift in der Datei, mit der **jede Session startet**. **AFK.**

## What to build

CLAUDE.md beansprucht ausdrücklich, Projekt-Narrativ, Struktur und **Baustand** zu tragen,
führt aber zwei überholte Angaben:

- Sie spricht von „vierzehn ADRs" und listet bis 0014, obwohl
  [ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) (generischer
  Slice-Store über `StoreSpec`) existiert und die Tickets 0074–0077 darauf aufbauen.
- Sie führt die `obfuskierung`-Ergebnis-Stores unter „Deferred … still in-memory doubles",
  obwohl der `obfuskierung`-Store seit
  [0075](0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md) real gegen
  SQLite läuft.

Weil jede Session mit dieser Datei als Kontext beginnt, ist eine falsche Baustandsangabe
dort kein Schönheitsfehler, sondern führt Agenten und Menschen gleichermaßen in die Irre.
Der Slice zieht Zählung, ADR-Liste und den Status-/Deferred-Abschnitt auf den tatsächlichen
Stand und bringt die Build-order-Sektion mit demselben Stand in Deckung.

## Acceptance criteria

- [x] ADR-Zählung und ADR-Liste führen ADR-0015 als tragende Entscheidung.
- [x] Der Status-/Deferred-Abschnitt trennt die **real gebauten** Stores von den
      verbleibenden In-Memory-Doubles. **Befund beim Umsetzen:** die Annahme des Tickets
      (`fenstern`/`auswerten` noch als Double) war beim Schreiben schon überholt — die
      Issues 0057/0058 haben auch diese beiden Stores real gebaut, ebenso **alle**
      Lese-Quellen. Real ist damit der Store **jedes** Slice; ohne realen io-Adapter sind
      nur noch der Eigene-Konten-Loader (0002), `Speicher`/Kategorie-Register samt
      `Kategorisierer`-Orakel und der `report`.
- [x] Die Build-order-Sektion nennt denselben Stand wie der Status-Abschnitt — kein
      Widerspruch zwischen zwei Stellen derselben Datei.
- [x] Jede Aussage in CLAUDE.md über einen **realen oder noch fehlenden io-Adapter** ist
      gegen den Code geprüft und trifft zu (Store je Slice, Config-Loader, Quelle). Dabei
      mitgezogen: die beiden „`io` später"-Stellen im `einlesen`-Absatz
      (`Calendar`/`Money`-Seams und die CSV-Quelle sind real) und der Prerequisites-Absatz
      (der Seed-**Loader** ist real, offen ist nur noch der Seed-**Wert**).
- [x] Alle sechs Gates grün.

## Blocked by

None - can start immediately.
