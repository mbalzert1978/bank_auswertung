---
id: "0012"
title: "HTML-Bericht"
status: open
---

# HTML-Bericht

**Typ:** AFK

> GitHub-Issue: [#99](https://github.com/mbalzert1978/bank_auswertung/issues/99)

## Was zu bauen ist

Der `Bericht`-Port (outbound), HTML zuerst (Baureihenfolge-Schritt 6): zeigt
die **Analyse der Domäne** — Umbuchungen (Slice 0004), Wiederkehr (Slice
0005), Gebühren (Slice 0006), Aderlass/Pareto/Spar-Kandidaten (Slice 0007) —
nicht „was das LLM sagte". Kategorie ist nur ein Attribut einer `Buchung`,
kein Berichtsinhalt für sich.

Bewusst **nicht** blockiert durch Speicher (Slice 0009) oder Kategorisierer
(Slice 0011): Bericht und Speicher sind unabhängige hexagonale Ports, und der
Lackmustest verlangt, dass das Programm auch ganz ohne LLM wertvoll bleibt —
der Bericht muss also auch mit unkategorisierten Buchungen sinnvoll
funktionieren.

> **Vorbedingung (Entscheidung des Orchestrators, 2026-10-12):** das Kategorie-Register aus
> [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md) ist die **einzige Wahrheit** für
> Kategorie-Etiketten. Der Bericht liest die Kategorie je Treiber **ausschließlich** über
> `io::auswerten::SqliteCategoryRegistry::categories_of` (fehlt ein Eintrag: „unbekannt"). Die
> Etiketten im Fenster-Ergebnis (`window_evaluation_categories`) sind nur der Mitschnitt dessen, was
> das Orakel in jenem Lauf beitrug — **keine** Quelle für den Bericht.

## Abnahmekriterien

- [x] HTML-Bericht rendert Umbuchungs-Übersicht, Wiederkehr-Liste (inkl.
      Jahreskosten/Preissteigerungen), Gebühren-Summen, Aderlass/Cashflow je
      Konto & Monat, Pareto-Ranking und Spar-Kandidaten aus den jeweiligen
      Analyse-Ergebnissen.
- [x] Buchungen ohne Kategorie erscheinen im Bericht (z. B. als „unbekannt"),
      der Bericht bricht nicht und verliert keine Analyse-Inhalte, wenn keine
      einzige Buchung kategorisiert ist.
- [x] Test/Fixture belegt: der Bericht zeigt errechnete Domänen-Werte
      (Summen, Rankings), nicht einen roh durchgereichten LLM-Text.
- [x] Bericht lässt sich aus reinen Fixture-Analyse-DTOs erzeugen, ganz ohne
      SQLite-Speicher oder Kategorisierer-Adapter im Testlauf.
- [x] Ist `RunReport::incomplete() > 0`, nennt der Bericht sichtbar, wie viele Buchungen der
      Analyse fehlen ([ADR-0017](../adr/0017-dead-letter-als-status-spalte-der-outbox.md)).
- [x] Jedes Kategorie-Etikett wird im HTML **escaped**: es stammt aus einer Chat-Antwort
      ([0011](0011-kategorisierer-chat-payload-adapter.md)) und ist nur auf Länge, Einzeiligkeit und
      Steuerzeichen geprüft, nicht auf HTML.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md)
- Blockiert durch [0006](0006-gebuehren-klassifikation.md)
- Blockiert durch [0007](0007-aderlass-pareto-ranking-und-spar-kandidaten.md)

## Wie es gebaut wurde

**Aufruf.** `bank-auswertung` hat eine achte **Pflicht**-Option `--bericht <datei.html>` — der Bericht
ist das Produkt, ein Lauf ohne ihn wäre die Ausnahme, nicht die Regel. Der Lauf selbst schreibt ihn
(`io::run`), die Bin-Crate bleibt reine Kommandozeile.

**Wann (ADR-0016).** Nur am Ende eines Laufs mit Exit 0. `RunReport::run_id()` gibt jetzt die
gemintete Lauf-Identität heraus, und der Bericht liest genau sie — nie `max(run_id)`. Geschrieben wird
in eine Nachbar-Datei `<ziel>.tmp`, die erst am Ende auf das Ziel umbenannt wird: nach Exit 1 bleibt
eine ältere Datei unberührt, ein halber Bericht liegt nie am Ziel. Scheitert das Schreiben, endet der
Lauf mit `RunError::Report` (Exit 1).

**Datenweg.**

- Der Abnehmer der fenster-übergreifenden Reichweite **hält** jetzt ihr Ergebnis fest, statt nur zu
  zählen (`RecordingHistorySink`, vorher `CountingHistorySink`). Welcher Lauf: Phase 1 arbeitet alle
  offenen Nachrichten ab (auch den Abschluss eines nach einem Absturz liegengebliebenen alten Laufs);
  der Lauf verwirft, was bis dahin ankam. In Phase 2 liegt nur noch der Anstoß **dieses** Laufs in der
  Outbox — was dann ankommt, ist sein Ergebnis. Cashflow je Konto und Monat, Pareto-Ranking,
  Spar-Kandidaten und Wiederkehr kommen von dort.
- Umbuchungen und Gebühren liest der Bericht aus `window_evaluation_transfers`/`_fees`, gescopt über
  `auswertungsfenster.run_id`. Die Fenster eines Laufs überlappen nicht, ein Monat kann aber in zwei
  Fenstern liegen; die Abfrage summiert darum je Monat und Rolle bzw. Konto.
- Die Kategorie je Treiber kommt **allein** aus `SqliteCategoryRegistry::categories_of`; fehlt ein
  Eintrag (und bei Gebühren und Abflüssen ohne Gegenpartei, die keinen Fingerabdruck haben):
  „unbekannt".
- Der Bericht rechnet nichts nach — kein Saldo, keine eigene Summe außer dem Zusammenzählen der
  Fenster-Teilstücke. Er beschriftet und formatiert.

**Wo der Code lebt.** `crates/io/src/report/`: `model.rs` (Bericht-DTO + reine Zusammensetzung aus
den Analyse-DTOs), `html.rs` (Seite, `escape`), `source.rs` (SQL des Laufs), `file.rs` (Nachbar-Datei
+ Umbenennen). Das Bericht-DTO trägt `assert_not_impl_any!(…: Serialize, Deserialize)`. **Keine eigene
Slice-Triade:** der Bericht trägt keine Domänen-Regel, nur Format-Wissen — das gehört nach `io` wie das
CSV-Format beim Einlesen; kein ADR verlangt eine Triade (YAGNI). Ebenso **kein `Bericht`-Port-Trait**:
es gibt genau einen Bericht-Adapter; ein Trait käme mit dem zweiten Format.

**Beschriftung (Privacy-Invarianten — die Report-Ausgabe ist ein Egress).**

- **Konten** über ihre **Rolle** aus der Eigene-Konten-Config (`Giro`, `Haushalt`, `Spar`, `Vorlauf`),
  nie über das `KONTO_…`-Token. Ohne Rolle: `Konto ohne Rolle`; mehrere Konten derselben Beschriftung
  werden durchnummeriert (`Giro 1`, `Giro 2`).
- **Firmen** über ihren **gescrubbten Firmennamen** aus `obfuskiert` (er ist dort schon ein bewusster
  Analyse-Anker, ADR-0010/`Recipient::Company`), sonst über die öffentliche **Gläubiger-ID**. Ein
  Händler-Token hinter einem Zahlungsdienst steht daneben (`ZAHLDIENST › HÄNDLER`).
- **Personen** nur über ihr seed-basiertes Pseudonym (`PERSON_…`).
- **Jeder** Text geht durch `escape` (`& < > " '`), Kategorie-Etiketten und Firmennamen eingeschlossen.
- Beträge in der Schreibweise der Währung (`1.234,56 €`, Nachkommastellen nach ISO 4217), Monate
  ausgeschrieben (`März 2026`), Anteile in Prozent mit zwei Stellen. Eine eigenständige Seite: CSS
  inline, keine externe Ressource, kein JavaScript.

**Belegt durch** (`crates/io/src/report/tests.rs` aus reinen Fixture-DTOs, `crates/io/src/run/bericht_tests.rs`
über echte Läufe):

- AK 1: `der_bericht_entsteht_aus_reinen_fixture_dtos_mit_allen_abschnitten`,
  `der_bericht_zeigt_die_errechneten_werte_in_euro_monaten_und_prozent`.
- AK 2: `ohne_eine_einzige_kategorie_steht_jeder_treiber_als_unbekannt_da`,
  `eine_leere_historie_ergibt_einen_bericht_ohne_eintraege`.
- AK 3: `die_kategorie_aendert_keine_zahl_sie_ist_nur_ein_attribut_des_treibers`,
  `der_bericht_zeigt_die_errechneten_werte_in_euro_monaten_und_prozent`.
- AK 4: alle Tests in `report/tests.rs` — kein SQLite, kein Kategorisierer.
- AK 5: `fehlen_der_analyse_buchungen_nennt_der_bericht_ihre_zahl_ganz_oben`,
  `ein_lauf_mit_fehlenden_buchungen_nennt_ihre_zahl_im_bericht`.
- AK 6: `ein_etikett_mit_markup_erscheint_escaped`,
  `die_kategorie_kommt_aus_dem_register_und_erscheint_escaped`.
- Datenschutz: `ein_echter_lauf_schreibt_einen_bericht_ohne_iban_bic_klarname_token_und_seed` (liest
  IBANs, BICs und Personen-Namen aus der Lauf-CSV selbst), `konten_erscheinen_ueber_ihre_rolle_nie_als_token`.
- ADR-0016: `der_bericht_zeigt_genau_den_eigenen_lauf_auch_neben_einem_frueheren`,
  `ein_gescheiterter_lauf_schreibt_keinen_bericht_und_laesst_den_alten_stehen`.

**Grenzen.** Der Firmenname ist so gut gescrubbt wie aller Freitext — heuristisch, nicht beweisbar
vollständig. Ein Fingerabdruck einer unbekannten Form erscheint unverändert (er trägt per
Domänen-Konstruktion nur öffentliche Anker oder Pseudonyme).
