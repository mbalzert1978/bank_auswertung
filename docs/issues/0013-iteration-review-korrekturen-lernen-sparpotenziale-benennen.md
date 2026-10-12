---
id: "0013"
title: "Iteration: Review, Korrekturen lernen, Sparpotenziale benennen"
status: open
---

# Iteration: Review, Korrekturen lernen, Sparpotenziale benennen

**Typ:** HITL — gemeinsame Review-Sitzung mit Markus, kein reiner Code-Slice.

> GitHub-Issue: [#100](https://github.com/mbalzert1978/bank_auswertung/issues/100)

## Was zu bauen ist

Baureihenfolge-Schritt 7, der Abschluss der ersten vollen
Tracer-Bullet-Runde: Markus geht den HTML-Bericht (Slice 0012) gemeinsam mit
Claude durch, korrigiert falsch zugeordnete Kategorien (die über den
Kategorisierer-Adapter, Slice 0011, als gelernte Korrekturen ins
Kategorie-Register, Slice 0009, einfließen) und benennt aus den
Spar-Kandidaten (Slice 0007) realistische Sparpotenziale.

Kein neuer Produktionscode-Pfad — das Ergebnis dieses Slices sind
festgehaltene Entscheidungen (gelernte Korrekturen, benannte
Sparpotenziale), nicht ein weiteres Feature.

## Abnahmekriterien

- [ ] Mindestens eine Kategorie-Korrektur aus der Review-Sitzung ist im
      Kategorie-Register (Slice 0009) gespeichert und wird bei einer erneuten
      Analyse automatisch angewendet.
- [ ] Mindestens ein aus dem Bericht abgeleitetes Sparpotenzial ist konkret
      benannt (Treiber + geschätzte Ersparnis), nicht nur pauschal
      „ausgabenärmer leben".
- [ ] Aus der Sitzung entstehende Nacharbeiten (z. B. falsch klassifizierte
      Wiederkehr-Muster, fehlende Scrubbing-Fälle) sind als konkrete
      Beobachtungen festgehalten, nicht nur mündlich besprochen.

## Blockiert durch

> **Aufgelöst (2026-10-12):** 0009, 0011 und 0012 sind gebaut. Ablauf der Sitzung:
> [Schritt-6-Checkliste](../plans/2026-10-12-schritt-6-checkliste.md), Punkt 7.

- Blockiert durch [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
- Blockiert durch [0011](0011-kategorisierer-chat-payload-adapter.md)
- Blockiert durch [0012](0012-html-bericht.md)
