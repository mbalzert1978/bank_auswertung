---
id: "0009"
title: "Speicher (SQLite) für DTOs und Kategorie-Register"
status: blocked
---

# Speicher (SQLite) für DTOs und Kategorie-Register

**Typ:** AFK

> **Hinweis zum Zuordnungs-Speicher:** Seit
> [ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md) gibt es **keinen**
> persistenten Klartext↔Pseudonym-Speicher mehr (Slice 0008 hinfällig). Das lokal zu
> haltende Geheimnis ist der **config-injizierte Seed**. Die Kern-Invariante unten bleibt:
> der `Speicher`-Port hält **ausschließlich obfuskierte DTOs** und **nie** den Seed/Rohwert;
> die AK-Punkte zur „Trennung vom Zuordnungs-Speicher (Slice 0008)" lesen sich heute als
> „der `Speicher` trägt den Seed nicht". Die Vorbedingung „setzt Slice 0008 voraus" entfällt.

## Was zu bauen ist

Der `Speicher`-Port (outbound), SQLite-Adapter (Baureihenfolge-Schritt 4):
persistiert **DTOs** (nicht Domänenobjekte) über mehrere Importe hinweg —
strikt getrennt vom Zuordnungs-Speicher aus Slice 0008 (nie mischen).

Enthält das **Kategorie-Register** (D5): schlüsselt auf dem `Fingerabdruck`,
speichert gelernte Korrekturen. Das Mapping DTO ↔ Domäne lebt im
Anti-Corruption-Layer, der das Kategorie-Register beim Laden aus den
gespeicherten DTOs rekonstituiert.

Setzt die persistente Pseudonym-Zuordnung (Slice 0008) voraus, damit
`PERSON_x`-Token schon beim ersten dauerhaft gespeicherten Import
laufstabil sind — sonst würde ein späterer Wechsel von laufstabilen zu
importübergreifenden Pseudonymen bereits gespeicherte Historie inkonsistent
machen.

## Abnahmekriterien

- [ ] `Buchung`-DTOs aus zwei separaten CSV-Importen werden in SQLite
      persistiert und beim Neustart des Programms vollständig wieder geladen.
- [ ] Das Kategorie-Register schlüsselt auf `Fingerabdruck`; eine gelernte
      Korrektur (Fingerabdruck → Kategorie) übersteht einen Neustart und wird
      beim nächsten Laden automatisch wieder angewendet.
- [ ] Der Anti-Corruption-Layer rekonstituiert beim Laden Domänenobjekte aus
      den gespeicherten DTOs, ohne dass die Domäne SQLite oder das
      Speicherformat kennt.
- [ ] Test: der `Speicher`-Port speichert ausschließlich obfuskierte DTOs —
      kein Test-Fixture mit Klartext-Namen landet unverändert in der
      SQLite-Datei.
- [ ] Test belegt die strukturelle Trennung von Zuordnungs-Speicher (Slice
      0008) und diesem `Speicher`: unterschiedliche Speicherorte/Formate,
      kein gemeinsamer Zugriffspfad.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0008](0008-persistente-pseudonym-zuordnung.md)
