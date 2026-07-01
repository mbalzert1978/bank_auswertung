---
id: "0008"
title: "Persistente Pseudonym-Zuordnung"
status: blocked
---

# Persistente Pseudonym-Zuordnung

**Typ:** AFK

## What to build

Der **Zuordnungs-Speicher** (lokal, persistent, verlässt den Rechner nie):
Klartext↔Pseudonym-Mapping, **getrennt** vom obfuskierten `Speicher`-Port aus
Slice 0009 — beide Speicher zu mischen ist ein Leak-Pfad und darf nicht
passieren.

First-Seen-Vergabe: ein Klarname, der zum ersten Mal auftaucht, bekommt ein
neues `PERSON_x`; einmal vergeben bleibt es fest. Anders als die
Ad-hoc-Pseudonymisierung in Slice 0003 (dort nur laufstabil innerhalb eines
Programmlaufs) ist diese Zuordnung **über mehrere Programmläufe/Importe
hinweg** stabil — sonst bricht der Fingerabdruck von
Personen-zu-Personen-Daueraufträgen, weil derselbe Mensch bei jedem Import ein
neues Pseudonym bekäme. Eigene Konten (`KONTO_x`) sind ohnehin stabil (aus der
Eigene-Konten-Config, Slice 0002) und brauchen dieses Mapping nicht.

## Acceptance criteria

- [ ] Ein Klarname, der zum ersten Mal eingespeist wird, erhält ein neues
      `Pseudonym` (`PERSON_x`); der Zuordnungs-Speicher wird lokal
      persistiert.
- [ ] Derselbe Klarname liefert bei einem **zweiten, separaten Programmlauf**
      (Zuordnungs-Speicher neu geladen) exakt dasselbe Pseudonym — Test
      startet zwei getrennte Instanzen gegen denselben persistierten Speicher.
- [ ] Der Zuordnungs-Speicher ist strukturell getrennt vom `Speicher`-Port
      (Slice 0009) — kein gemeinsamer Datentyp, keine gemeinsame Tabelle/Datei.
- [ ] Es existiert kein Pfad, über den ein Klartext-Name aus diesem Speicher
      in ein DTO oder in den `Speicher`-Port (obfuskierte Domäne) gelangt.

## Blocked by

- Blocked by [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
