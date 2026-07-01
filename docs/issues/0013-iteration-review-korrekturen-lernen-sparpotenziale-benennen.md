---
id: "0013"
title: "Iteration: Review, Korrekturen lernen, Sparpotenziale benennen"
status: blocked
---

# Iteration: Review, Korrekturen lernen, Sparpotenziale benennen

**Typ:** HITL — gemeinsame Review-Sitzung mit Markus, kein reiner Code-Slice.

## What to build

Baureihenfolge-Schritt 7, der Abschluss der ersten vollen
Tracer-Bullet-Runde: Markus geht den HTML-Bericht (Slice 0012) gemeinsam mit
Claude durch, korrigiert falsch zugeordnete Kategorien (die über den
Kategorisierer-Adapter, Slice 0011, als gelernte Korrekturen ins
Kategorie-Register, Slice 0009, einfließen) und benennt aus den
Spar-Kandidaten (Slice 0007) realistische Sparpotenziale.

Kein neuer Produktionscode-Pfad — das Ergebnis dieses Slices sind
festgehaltene Entscheidungen (gelernte Korrekturen, benannte
Sparpotenziale), nicht ein weiteres Feature.

## Acceptance criteria

- [ ] Mindestens eine Kategorie-Korrektur aus der Review-Sitzung ist im
      Kategorie-Register (Slice 0009) gespeichert und wird bei einer erneuten
      Analyse automatisch angewendet.
- [ ] Mindestens ein aus dem Bericht abgeleitetes Sparpotenzial ist konkret
      benannt (Treiber + geschätzte Ersparnis), nicht nur pauschal
      „ausgabenärmer leben".
- [ ] Aus der Sitzung entstehende Nacharbeiten (z. B. falsch klassifizierte
      Wiederkehr-Muster, fehlende Scrubbing-Fälle) sind als konkrete
      Beobachtungen festgehalten, nicht nur mündlich besprochen.

## Blocked by

- Blocked by [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
- Blocked by [0011](0011-kategorisierer-chat-payload-adapter.md)
- Blocked by [0012](0012-html-bericht.md)
