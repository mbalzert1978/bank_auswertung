---
id: "0088"
title: "Zeilen-Schlüssel, lauf-eigene Fenster und vorgemerkte Umsätze überspringen"
status: open
---

# Zeilen-Schlüssel, lauf-eigene Fenster und vorgemerkte Umsätze überspringen

**Typ:** AFK

> GitHub-Issue: [#105](https://github.com/mbalzert1978/bank_auswertung/issues/105)

## What to build

Mehrere Läufe in **eine** Datenbank — je eigenem Konto einer, auch wiederholt — sollen
richtige Zahlen ergeben. Heute tun sie das nicht, aus drei Gründen:

1. **Kein Zeilen-Schlüssel.** Alle Ergebnis-Tabellen haben nur eine generierte `id`
   (`crates/io/src/einlesen/store.rs`, `crates/io/src/fenstern/store.rs`). Wer dieselbe CSV
   zweimal einliest, hält den Bestand zweimal.
2. **Fenster sind nicht an einen Lauf gebunden.** `fenstern` paginiert je Lauf die **ganze**
   obfuskierte Historie neu, und die fenster-übergreifende Abfrage
   (`crates/io/src/auswerten/source.rs`) scopt nur über das Datum. Fenster früherer Läufe zählen
   deshalb erneut mit (gemessen: 53 → 159 → 318 Fenster-Buchungen über drei Läufe).
3. **Vorgemerkte Zeilen brechen den Import.** Eine Zeile mit `Info` = `Umsatz vorgemerkt` hat
   ein leeres Valutadatum; `value_date` ist Pflichtfeld
   (`crates/einlesen_adapter/src/models/raw_record.rs`), also scheitert die **ganze** CSV
   terminal. Die Zeile trägt außerdem `0` als IBAN/BIC und keine Kundenreferenz.

Die Entscheidung dazu hält **ADR-0016** fest (Teil dieses Tickets). Ihr Rahmen ist im
[Abschlussplan](../plans/2026-10-12-1343-abschlussplan.md), Schritt 2, mit Markus abgestimmt:

- **Zeilen-Schlüssel** = Auftragskonto + Kundenreferenz (End-to-End), wenn gefüllt; sonst
  Rückfall auf Auftragskonto + alle Spalten + laufende Nummer gleicher Zeilen innerhalb des
  Imports. Grenze (im ADR benennen): ein Export darf keinen Tag mittendrin abschneiden.
- Ein neuer Import **ergänzt**; eine schon bekannte Zeile wird nicht erneut angelegt (und nicht
  erneut publiziert).
- Fenster und Auswertungen gehören zu genau **einem** Lauf; die fenster-übergreifende Abfrage
  scopt über den Lauf statt nur über das Datum.
- `einlesen` überspringt `Umsatz vorgemerkt` **vor** der Pflichtfeld-Prüfung.

Die Schema-Änderung ist die **erste echte Migration** auf dem Versionsanker aus
[0084](0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md).

## Acceptance criteria

- [ ] ADR-0016 hält die vier Entscheidungen oben fest (inkl. der Grenze des Rückfall-Schlüssels).
- [ ] Test mit erfundenen Daten: CSV A, dann CSV B, dann A erneut in eine Datenbank ergibt
      dieselben Zahlen wie A + B je einmal.
- [ ] Test: zwei gleiche Buchungen am selben Tag (ohne Kundenreferenz) zählen beide.
- [ ] Test: eine `Umsatz vorgemerkt`-Zeile bricht den Import nicht ab und zählt nicht.
- [ ] Test: ein zweiter Lauf zählt keine Fenster eines früheren Laufs mit.
- [ ] Die Schema-Version steigt; die Migration läuft über den Anker aus 0084.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- [0084](0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md) — braucht den
  Migrations-Anker.
