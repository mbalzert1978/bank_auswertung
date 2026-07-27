---
id: "0080"
title: CLAUDE.md-Baustand auf ADR-0015 und die realen Stores nachziehen
status: open
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

- [ ] ADR-Zählung und ADR-Liste führen ADR-0015 als tragende Entscheidung.
- [ ] Der Status-/Deferred-Abschnitt trennt die **real gebauten** Stores (`einlesen`,
      `obfuskierung`) von den verbleibenden In-Memory-Doubles (`fenstern`, `auswerten`).
- [ ] Die Build-order-Sektion nennt denselben Stand wie der Status-Abschnitt — kein
      Widerspruch zwischen zwei Stellen derselben Datei.
- [ ] Jede Aussage in CLAUDE.md über einen **realen oder noch fehlenden io-Adapter** ist
      gegen den Code geprüft und trifft zu (Store je Slice, Config-Loader, Quelle).

## Blocked by

None - can start immediately.
