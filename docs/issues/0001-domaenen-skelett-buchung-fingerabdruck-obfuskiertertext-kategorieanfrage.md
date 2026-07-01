---
id: "0001"
title: "Domänen-Skelett: Buchung, Fingerabdruck, ObfuskierterText, KategorieAnfrage"
status: closed
---

# Domänen-Skelett: Buchung, Fingerabdruck, ObfuskierterText, KategorieAnfrage

**Typ:** AFK

## What to build

Der erste Tracer-Bullet-Slice (Baureihenfolge-Schritt 1): das Domänen-Skelett
als Cargo-Workspace-Domänen-Crate, rein von Value Objects bis zur schmalen
`KategorieAnfrage`-Ausgangs-DTO.

Value Objects (parse, don't validate): `Betrag` (vorzeichenlos + Währung),
`KontoId`, `Pseudonym`, `Fingerabdruck`, `GläubigerId`, `ObfuskierterText`
(minimale Erstversion — Scrubbing-Regeln werden erst in einem späteren Slice
vertieft). Summentypen mit erschöpfendem `match`: `Richtung::Eingang|Ausgang`,
`Gegenkonto::Eigen|Fremd`, `Empfänger::Firma|Person`, `Buchungsart`. `Betrag`
ist vorzeichenlos; das Vorzeichen lebt ausschließlich in `Richtung`.

`Fingerabdruck` ist eine reine Funktion `Buchung → Fingerabdruck` — hier
zunächst eine Basisversion (Deduplizierung gleichartiger Buchungen); die
Zahlungsintermediär-Erkennung (PayPal/Klarna/Amazon Payments als
Durchleiter statt Händler) folgt vertieft in einem eigenen späteren Slice.

`Buchung` setzt sich aus diesen Value Objects zusammen. Die Projektion
`Buchung → KategorieAnfrage` (Fingerabdruck + `ObfuskierterText` für
Empfänger/Verwendungszweck) ist die schmale, eigene DTO des
`Kategorisierer`-Ports — kein Gott-DTO.

Alle Konstruktoren validieren einmal am Rand; ein illegaler Wert liefert kein
Objekt. Stil: deklarativ-funktional, reine Transformationen über unveränderliche
Werte.

## Acceptance criteria

- [ ] `Betrag`, `KontoId`, `Pseudonym`, `Fingerabdruck`, `GläubigerId`,
      `ObfuskierterText` existieren als Value Objects mit validierenden
      Konstruktoren; ein illegaler Wert (z. B. negativer `Betrag`) ist nicht
      konstruierbar (Test: Konstruktor liefert `None`/`Err`, kein „Objekt da,
      aber innen kaputt").
- [ ] `Richtung::Eingang|Ausgang`, `Gegenkonto::Eigen|Fremd`,
      `Empfänger::Firma|Person`, `Buchungsart` sind Summentypen mit
      erschöpfendem `match`; eine negative `Eingang`-Buchung ist nicht
      konstruierbar.
- [ ] `Empfänger`-Auflösung ist fail-safe: bei Unsicherheit „Firma vs.
      Privatperson" wird als `Person` behandelt (Test deckt den
      Unsicherheits-Fall ab).
- [ ] `Buchung` lässt sich aus den Value Objects konstruieren.
- [ ] Die Projektion `Buchung → KategorieAnfrage` ist implementiert.
- [ ] **Tragender Invariant-Test**: die Projektion `Buchung → KategorieAnfrage`
      trägt **keinen strukturellen Identifikator** — `Iban`/`Bic` sind ohne
      `Serialize` und in keinem DTO einbettbar (ein Einbettungsversuch ist ein
      Compile-Fehler, nicht nur ein Laufzeit-Test).
- [ ] `cargo test` und `cargo clippy --all-targets` laufen grün gegen den
      Domänen-Crate ohne I/O-Abhängigkeit (kein `reqwest`, kein `rusqlite`).

## Blocked by

None - can start immediately.
