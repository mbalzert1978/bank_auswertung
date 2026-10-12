---
id: "0010"
title: "Scrubbing-Regeln für ObfuskierterText"
status: closed
---

# Scrubbing-Regeln für ObfuskierterText

**Typ:** AFK

> GitHub-Issue: [#97](https://github.com/mbalzert1978/bank_auswertung/issues/97)

## Was zu bauen ist

Vertieft `ObfuskierterText` (D3, Slice 0001 lieferte nur eine minimale
Erstversion) um die eigentlichen Scrubbing-Regeln: in Freitext
(Empfänger/Verwendungszweck) eingebettete Identifikatoren (IBAN-/BIC-Muster,
bekannte Personen-Token) werden **best-effort** gescrubbt, bevor Text
überhaupt als `ObfuskierterText` konstruierbar ist — „ungescrubbt" ist gar
nicht serialisierbar.

Fail-safe: im Zweifel wird eher zu viel als zu wenig gescrubbt. Das Scrubbing
ist **heuristisch, nicht beweisbar vollständig** — das ist eine ehrliche
Grenze des Modells, keine Lücke, die dieser Slice schließen soll (siehe PRD
„Out of Scope").

## Abnahmekriterien

- [x] Eine im Verwendungszweck eingebettete IBAN wird vom Konstruktor erkannt
      und ersetzt/entfernt, bevor der Text als `ObfuskierterText` existiert.
      → `eine_iban_im_verwendungszweck_wird_ersetzt`,
      `eine_zusammengeklebte_iban_wird_auch_ohne_wortgrenze_ersetzt`,
      `eine_klein_geschriebene_auslaendische_iban_mit_buchstaben_gruppe_wird_ersetzt`
- [x] Eine im Freitext eingebettete BIC wird ebenso gescrubbt.
      → `eine_bic_im_verwendungszweck_wird_ersetzt`,
      `eine_auslaendische_bic_wird_hinter_dem_hinweis_bic_ersetzt`
      (Gegenprobe: `grossgeschriebene_woerter_in_bic_form_bleiben_stehen`)
- [x] Ein bekanntes Personen-Token im Freitext wird gescrubbt (fail-safe:
      Unsicherheitsfall wird lieber entfernt als riskiert).
      → `ein_bekannter_personen_name_wird_ersetzt`,
      `ein_bekannter_name_wird_auch_gross_geschrieben_und_in_bank_schreibweise_ersetzt`,
      `ein_bekannter_name_im_genitiv_wird_ersetzt`,
      `eine_firma_ohne_gläubiger_id_auf_der_namensliste_wird_mitgeschwärzt`
      (Gegenprobe der bewussten Lücke: `ein_vorname_allein_eines_bekannten_empfaengers_bleibt_stehen`);
      Herkunft der Namen real belegt durch
      `liefert_empfaenger_ohne_glaeubiger_id_und_die_konfigurierten_namen` (io) und
      `ein_lauf_schwaerzt_namen_iban_und_bic_im_verwendungszweck_bevor_sie_obfuskiert_liegen` (Lauf)
- [x] Test-Suite mit mehreren realistischen (aber synthetischen) Freitext-
      Beispielen, die je einen Identifikator-Typ enthalten, belegt die
      Scrubbing-Regeln einzeln.
      → Abschnitt „Scrubbing" in `crates/obfuskierung_specs/tests/obfuskierung.rs`
      (IBANs mit Prüfziffer `00`, Fantasie-Bank `FIKT`, erfundene Namen)
- [x] Dokumentiert (Docstring/Kommentar am Konstruktor): das Scrubbing ist
      heuristisch und nicht beweisbar vollständig — keine
      Vollständigkeits-Behauptung im Code oder in Tests.
      → `ObfuscatedText::scrub` und Modul `scrubbing` in `obfuskierung_domain`;
      Gegenprobe im Test `ein_unbekannter_name_bleibt_stehen_das_scrubbing_ist_nicht_vollstaendig`

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)

## Wie es gebaut wurde

Regel V4 aus [Plan 2026-10-12-1343](../plans/2026-10-12-1343-abschlussplan.md), Schritt 3b.

- **Ein Konstruktor, kein Umweg.** `ObfuscatedText` hat nur noch das crate-interne
  `ObfuscatedText::scrub(text, &KnownPersons)`; das alte `From<NonEmptyString>` ist weg. Auch der
  Firmenname einer Firma mit Gläubiger-ID läuft durch denselben Engpass (bleibt Klartext, wird
  nicht pseudonymisiert, aber gescrubbt).
- **Ersatz durch feste Platzhalter** `[IBAN]`, `[BIC]`, `[PERSON]` — kein seed-basiertes
  `PERSON_x`. Grund: ADR-0010 hält fest, dass Freitext nicht seed-tokenisiert wird; ein fester
  Platzhalter verrät auch nicht, dass zwei Zwecke dieselbe Person nennen, und braucht weder Seed noch
  Obfuskator. Der Seed kommt so gar nicht erst in die Nähe des Texts.
- **IBAN:** zwei Buchstaben + zwei Ziffern, dann Buchstaben/Ziffern am Stück oder in Gruppen mit je
  einem Leerzeichen; ab 15 Zeichen. Prüfziffer wird bewusst nicht gerechnet (auch eine vertippte
  IBAN fällt). Gesucht wird auch mitten im Wort (`IBANDE00…`). Folgegruppen zählen, wenn sie eine
  Ziffer tragen oder aus vier Buchstaben in der Schreibweise des Ländercodes bestehen (`GB00 FIKT`,
  klein `gb00 fikt`) — so endet die IBAN vor dem nächsten Wort.
- **Als Lexer gebaut:** der Scrubber zerlegt den Text über den Iterator `Segments` in
  `Segment::Kept(char)`/`Segment::Replaced(Identifier)` und setzt ihn per
  `.map(Segment::render).collect()` wieder zusammen; ein Platzhalter wird nie erneut geprüft.
  `ObfuscatedText` bleibt im Typ nicht-leer (`NonEmptyString` über einen crate-internen, unfehlbaren
  Konstruktor nur für das Scrubbing-Ergebnis).
- **BIC:** ganzes Wort, 8 oder 11 Zeichen, sechs Großbuchstaben + Großbuchstaben/Ziffern. Die Form
  allein träfe `LASTSCHRIFT`, `PRIVATKONTO`, `KREDITKARTE` — darum zusätzlich **eins** von: Ziffer
  in Ort/Filiale, Land `DE`, oder ein `BIC`/`SWIFT`-Hinweis davor. Bekannte Lücke: ausländische BIC
  ohne Ziffer und ohne Hinweis. Bewusster Preis dafür, dass die Buchungsarten lesbar bleiben (an
  ihnen hängt die Gebühren-Klassifikation in `auswerten`). Gegenprobe am Lauf-Bestand: kein
  Fehltreffer.
- **Namen:** groß/klein egal, Leerraum/Komma zwischen den Teilen beliebig, auch „Nachname, Vorname",
  Genitiv-`s` zählt mit, nur ganze Wörter (`Rose` ≠ `Rosenmontag`), längster Name zuerst. Keine
  Umschriften (`Müller`/`Mueller`), keine Initialen.
- **Teilnamen — Entscheidung (Review-Runde 1):** Teilnamen eines Empfängers (nur der Vorname, der
  Name mit/ohne Titel, eine Hälfte eines Doppelnamens) werden **nicht** automatisch geschwärzt. V4
  nennt ganze Namen, und einzelne Namensteile träfen zu viel gewöhnlichen Text (`Klein`, `Lange`,
  `GmbH`). Stattdessen trägt der Mensch Vornamen/Kurzformen in `personen.toml` ein; Vorlage und
  `--help` sagen das.
- **Woher die Namen kommen:** die Domäne bekommt sie als Wert `KnownPersons` über den neuen Port
  `KnownPersonsSource` (Adapter-Seam `KnownPersonsSeam`). `io` erfüllt ihn mit
  `io::obfuskierung::KnownPersonNames`: `SELECT DISTINCT recipient_name FROM einlesen_lines` für
  Zeilen **ohne Gläubiger-ID** plus die Namen aus `personen.toml` (`io::config::PersonenConfig`).
  Gelesen wird **je Buchung**, damit auch die Namen des laufenden Imports schon zählen.
  - **Warum nur Empfänger ohne Gläubiger-ID:** genau die deutet der Slice fail-safe als Person.
    Firmen mit Gläubiger-ID auf die Liste zu setzen, hätte ihren Namen (das wichtigste
    Kategorie-Signal) in jedem Text geschwärzt — auch im eigenen Firmennamen-Feld. Die im Plan
    genannten Kosten bleiben: eine Firma **ohne** Gläubiger-ID landet auf der Liste und wird
    mitgeschwärzt.
  - Keine Klartext↔Token-Zuordnung (ADR-0010): nur Namen, kein Token, kein Seed, nichts abgelegt.
- **Fehler, redigiert:** neue Variante `ObfuscationError::PersonsUnavailable { reason }` (nur
  io-Diagnose, nie ein Name) — terminal, statt still mit leerer Liste zu scrubben. Neue
  `ConfigError`-Varianten `PersonNamesMissing`/`PersonNamesInvalid` zitieren keinen Eintrag (auch ein
  Eintrag ohne Namensteil wie `","` ist `PersonNamesInvalid`);
  `Debug` von `KnownPersons`, `PersonenConfig` und `KnownPersonNames` zählt nur.
- **Kommandozeile geändert:** `bank-auswertung` hat jetzt **sechs** Pflicht-Optionen, neu
  `--personen <datei>`. Entscheidung: Pflicht, auch wenn leer (`namen = []`). Fail-safe — eine
  vergessene Liste soll den Lauf laut stoppen, nicht still weniger scrubben. `--help`-Text,
  CLAUDE.md-Befehlszeile und Tests sind nachgezogen. Fixture heißt `crates/io/fixtures/lauf/namen.toml`
  (nicht `personen.toml`, weil der Name per .gitignore überall ausgeschlossen ist).

## Bewusste Grenzen

- **Heuristisch, nicht beweisbar vollständig.** Ein Name, der weder vollständig Empfänger (ohne
  Gläubiger-ID) war noch in `personen.toml` steht, bleibt stehen; ebenso Umschriften, Initialen,
  verfremdete IBANs und ausländische BICs ohne Ziffer und ohne `BIC`-Hinweis.
- **Teilnamen fallen nicht von selbst** (siehe oben) — sie gehören in `personen.toml`.
- **Namen wirken nicht rückwirkend.** Eine Buchung wird mit den Namen gescrubbt, die bei ihrer
  Obfuskierung bekannt sind. Taucht ein Empfänger erst in einem späteren Import auf, bleibt sein Name
  in schon obfuskierten Verwendungszwecken früherer Läufe stehen; einen Re-Scrub gibt es bewusst
  nicht. **Bedienregel:** vor dem ersten Lauf alle bekannten Empfänger-Namen aller Konten (inkl.
  Vornamen/Kurzformen) in `personen.toml` eintragen.
- **Händler ohne Gläubiger-ID.** Kartenzahlungs-Händler haben keine Gläubiger-ID und landen darum auf
  der Namensliste. Taucht ihr Name im Verwendungszweck eines Durchleiters auf (PayPal & Co.), wird er
  `[PERSON]` — und zwei solche Händler hinter demselben Durchleiter fallen auf **denselben
  Fingerabdruck** (der Händler-Token des Fingerabdrucks entsteht aus dem gescrubbten Zweck). Bewusst
  so gelassen: fail-safe vor Kategorie-Schärfe.
- **Aufwand je Lauf.** Die Namensliste wird je Buchung ganz gelesen, über einen Lauf also O(N²)
  Zeilen; für ein Privatkonto unkritisch. Ausweg bei Bedarf: einmal je Lauf nach dem Import lesen.
