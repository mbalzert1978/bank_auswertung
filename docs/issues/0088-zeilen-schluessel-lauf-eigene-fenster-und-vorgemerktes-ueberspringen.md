---
id: "0088"
title: "Zeilen-Schlüssel, lauf-eigene Fenster und vorgemerkte Umsätze überspringen"
status: closed
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

- [x] ADR-0016 hält die vier Entscheidungen oben fest (inkl. der Grenze des Rückfall-Schlüssels) —
      [docs/adr/0016](../adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md), dazu die Migration.
- [x] Test mit erfundenen Daten: CSV A, dann CSV B, dann A erneut in eine Datenbank ergibt
      dieselben Zahlen wie A + B je einmal —
      `run::tests::a_dann_b_dann_a_erneut_ergibt_dieselben_zahlen_wie_a_und_b_je_einmal`.
- [x] Test: zwei gleiche Buchungen am selben Tag (ohne Kundenreferenz) zählen beide —
      `run::tests::zwei_gleiche_buchungen_zaehlen_beide_und_eine_vorgemerkte_zeile_bricht_nichts`,
      `einlesen::store::tests::zwei_gleiche_buchungen_ohne_kundenreferenz_werden_beide_abgelegt`,
      `einlesen::source::tests::zwei_gleiche_zeilen_ohne_kundenreferenz_bekommen_verschiedene_schluessel`.
- [x] Test: eine `Umsatz vorgemerkt`-Zeile bricht den Import nicht ab und zählt nicht —
      Spec `eine_vorgemerkte_zeile_bricht_den_import_nicht_ab_und_zaehlt_nicht`,
      `einlesen::store::tests::eine_vorgemerkte_zeile_bricht_den_import_nicht_ab_und_wird_nicht_abgelegt`,
      `run::tests::zwei_gleiche_buchungen_zaehlen_beide_und_eine_vorgemerkte_zeile_bricht_nichts`.
- [x] Test: ein zweiter Lauf zählt keine Fenster eines früheren Laufs mit —
      `run::tests::ein_zweiter_lauf_zaehlt_keine_fenster_eines_frueheren_laufs_mit`,
      `auswerten::source::tests::history_in_sieht_nur_die_fenster_des_eigenen_laufs`,
      Spec `ein_spaeterer_lauf_zaehlt_die_fenster_eines_frueheren_nicht_mit`.
- [x] Die Schema-Version steigt; die Migration läuft über den Anker aus 0084 —
      `schema::tests::eine_version_1_datei_ohne_bestand_wird_auf_version_2_migriert`,
      `schema::tests::eine_version_1_datei_mit_bestand_wird_terminal_redigiert_abgewiesen_und_bleibt_unangetastet`.
- [x] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- [0084](0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md) — braucht den
  Migrations-Anker.

## Wie es gebaut wurde

- **Zeilen-Schlüssel (`einlesen`).** Die CSV-Quelle in `io` (`crates/io/src/einlesen/source.rs`)
  leitet je **gebuchter** Zeile den Schlüssel ab (`RowIdentity`): Auftragskonto + Kundenreferenz +
  Betrag + Buchungstag, wenn die Referenz gefüllt ist (`NOTPROVIDED` zählt als leer), sonst alle
  Spalten; dazu die laufende Nummer gleicher gebuchter Zeilen in der Datei. Eine vorgemerkte Zeile
  bekommt keinen Schlüssel und verschiebt keine Nummer. Abgelegt wird nur ein SipHash-128-Digest mit
  festem Schlüssel in `RawRecord.row_key`. Der Slice trägt ihn als `RowKey` durch; die ACL weist
  doppelte Schlüssel im Batch ab (`SourceError::DuplicateRowKey`) und verlangt ihn als Pflichtfeld.
  `einlesen_lines.row_key` ist `NOT NULL UNIQUE`.
- **Ein neuer Import ergänzt.** Der `Store`-Port antwortet `Recording::New(id) | AlreadyKnown`
  (Seam: `StoreOutcome`); der Handler publiziert nur Neues. Der io-Store fragt vorher den committeten
  Bestand (`SliceUnit::read`, neu und generisch).
- **Vorgemerktes.** `RawRecord.status` (`RecordStatus::Booked | Pending`), gesetzt aus der
  `Info`-Spalte; die `SourceBridge` verwirft `Pending` vor allen Prüfungen.
- **Lauf-eigene Fenster.** `io::run` mintet je Analyse eine Lauf-Identität und reiht sie als
  `partition_key` des `StartAnalysis` ein. `fenstern` trägt sie als `RunId` durch `AdvanceWindow`
  (`"{anchor}:{run_top}:{run}"`) in jedes `AnalysisWindow` (`auswertungsfenster.run_id`, indiziert)
  und in den Abschluss (`"{from}:{to}:{run}"`). In `auswerten` liest der Decoder den Lauf, die
  Lese-Bridge legt ihn als Scope an `OccurrenceSeam::history_in(run, from, to)`; die Abfrage scopt
  `WHERE w.run_id = ? AND` Spanne. Die `auswerten`-Domäne bleibt lauf-blind.
- **Migration 1 → 2** in `crates/io/src/schema.rs`: leere Version-1-Datei → die beiden geänderten
  Tabellen neu angelegt (Schema danach identisch mit einer frischen Datei); mit Bestand → terminal,
  redigiert abgewiesen (`MismatchCause::UnkeyedRows`), weil die Altzeilen nicht nachschlüsselbar sind.
- **Fixtures (erfunden):** `lauf_haushalt.csv` (zweites eigenes Konto, zwei gleiche Buchungen, eine
  vorgemerkte), `zwei_gleiche.csv`, `vorgemerkt.csv`, `kundenreferenz.csv`.
- **Doku:** CLAUDE.md und der `lauf`-Hilfetext beschreiben jetzt „mehrere Läufe, eine `--db`" samt
  der Grenze statt der alten „frische `--db`"-Warnung.

### Review-Runde 1

- **Datenverlust behoben:** der Kundenreferenz-Schlüssel trägt jetzt den Buchungstag (Etikett
  `kundenreferenz/2`); wiederkehrende Referenzen zum selben Betrag in überlappenden Auszügen verlieren
  keinen Monat mehr —
  `einlesen::store::tests::zwei_ueberlappende_auszuege_mit_wiederkehrender_referenz_verlieren_keinen_monat`,
  `einlesen::source::tests::ueberlappende_auszuege_mit_wiederkehrender_referenz_schluesseln_jeden_monat_eigen`.
- Nur gebuchte Zeilen werden durchnummeriert —
  `einlesen::source::tests::eine_vorgemerkte_zeile_bekommt_keinen_schluessel_und_verschiebt_keine_nummer`.
- Identität als Summentyp `RowIdentity`; Lauf-Scope in `auswerten` als Newtype `RunScope`.
- „Jüngster Lauf" = größte `run_id` (beginnt mit dem UTC-Startzeitpunkt) —
  `run::tests::der_juengste_lauf_ist_der_mit_der_groessten_lauf_identitaet`.
- Die Vorkommens-Zahl im Lauf-Bericht zählt nur den neuen Lauf.
- ADR-0012/0013 tragen „verfeinert durch ADR-0016".

