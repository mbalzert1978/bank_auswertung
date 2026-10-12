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

**Was er ist** ([ADR-0018](../adr/0018-bericht-als-io-formatierer-am-ende-eines-laufs.md)): ein
`io`-Formatierer am Ende eines Exit-0-Laufs — kein Outbox-Consumer, kein eigener Port, keine eigene
Crate oder Slice-Triade. Er rechnet nichts; HTML ist Format-Wissen.

**Wann (ADR-0016).** Nur am Ende eines Laufs mit Exit 0. `RunReport::run_id()` gibt die gemintete
Lauf-Identität heraus, und der Bericht liest genau sie — nie `max(run_id)`. Geschrieben wird in eine
Nachbar-Datei `<ziel>.tmp`, nach `sync_all` auf das Ziel umbenannt: nach Exit 1 bleibt eine ältere
Datei unberührt, ein halber Bericht liegt nie am Ziel. Scheitert das Schreiben, endet der Lauf mit
`RunError::Report` (Exit 1).

**Datenweg — eine Wahrheitsquelle für alle Zahlen.**

- `HistoryEvaluation` (Domäne, fenster-übergreifende Reichweite) liefert jetzt neben dem Cashflow je
  Konto und Monat auch die **Umbuchungen** je Monat und Gegenkonto-Rolle und die **Gebühren** je Monat
  und belastetem Konto über den ganzen Lauf. Die Bucketing-Funktionen teilen sich beide Reichweiten
  (`auswerten_domain::value_objects::monthly_views`); `HistoryEvaluationRow` trägt `transfers` und
  `fees` (Specs in `auswerten_specs/tests/aderlass.rs`).
- Der Abnehmer der Reichweite **hält** ihr Ergebnis fest (`RecordingHistorySink`, vorher
  `CountingHistorySink`). Welcher Lauf: Phase 1 arbeitet alle offenen Nachrichten ab (auch den
  Abschluss eines nach einem Absturz liegengebliebenen alten Laufs); der Lauf verwirft, was bis dahin
  ankam. In Phase 2 liegt nur noch der Anstoß **dieses** Laufs in der Outbox.
- Aus der Datei liest der Bericht allein die **Firmennamen** der Buchungen des Laufs, gescopt über
  `auswertungsfenster.run_id`.
- Die Kategorie je Treiber kommt **allein** aus `SqliteCategoryRegistry::categories_of`; fehlt ein
  Eintrag (und bei Gebühren und Abflüssen ohne Gegenpartei, die keinen Fingerabdruck haben):
  „unbekannt".
- Der Bericht beschriftet, ordnet und formatiert — er rechnet nichts nach.

**Wo der Code lebt.** `crates/io/src/report/`: `model.rs` (Bericht-DTO + reine Zusammensetzung aus
den Analyse-DTOs), `html.rs` (Seite, `escape`, ein `TableRow<N>`-Trait je Zeilentyp: Spalten und Zellen
passen per Typ zusammen, eine falsche Zellenzahl ist ein Compile-Fehler), `source.rs` (SQL der
Firmennamen), `file.rs` (Nachbar-Datei + `sync_all` + Umbenennen). Das Bericht-DTO trägt
`assert_not_impl_any!(…: Serialize, Deserialize)`.

**Beschriftung (Privacy-Invarianten — die Report-Ausgabe ist ein Egress).**

- **Konten** über ihre **Rolle**, wie die Domäne sie am Cashflow je Konto führt (`Giro`, `Haushalt`,
  `Spar`, `Vorlauf`), nie über das `KONTO_…`-Token. Ohne Rolle: `Konto ohne Rolle`; mehrere Konten
  derselben Beschriftung werden durchnummeriert (`Giro 1`, `Giro 2`).
- **Firmen** über ihren **gescrubbten Firmennamen** aus `obfuskiert` — aber **nur, wenn er keinen Namen
  der lokalen Namensliste (`personen.toml`) nennt**. Verglichen wird mit derselben Erkennung wie beim
  Scrubbing (`obfuskierung_adapter::names_known_person`, eine Regel statt zwei Kopien) — und nur, wenn
  er **keinen Scrub-Platzhalter** trägt (`obfuskierung_adapter::is_scrubbed`: stand der Name schon
  beim Obfuskieren auf der Liste, liegt er als `[PERSON]` bzw. `PRAXIS [PERSON]` in der Datei). Sonst,
  und ohne Namen, über die öffentliche **Gläubiger-ID**. Ein Händler-Token hinter einem Zahlungsdienst steht
  daneben (`ZAHLDIENST › HÄNDLER`).
- **Personen** nur über das Ende ihres Pseudonyms (`Person f30ee3`, sechs Zeichen — der
  seed-basierte Hash-Teil; ein festes Präfix muss der Bericht so nicht kennen). Teilen zwei Personen
  das Endstück, wird es verlängert, bis die Beschriftungen eindeutig sind.
- Beschriftet wird eine Buchungsart aus ihrem **Anker**: die Domäne gibt neben dem
  Fingerabdruck-Schlüssel die Struktur aus, aus der er entstand (`auswerten_domain::KindAnchor`:
  Firma mit Gläubiger-ID und optionalem Händler-Token / Person mit Pseudonym, gebildet aus der
  Einordnung `Party` aus 0011); der Adapter projiziert sie auf `KindAnchorRow` an
  `CostDriverRow::Kind`. `kind_label` ist ein erschöpfendes `match` — der Schlüssel wird nie zerlegt.
- **Jeder** Text geht durch `escape` (`& < > " '`), Kategorie-Etiketten und Firmennamen eingeschlossen.
- Beträge in der Schreibweise der Währung (`1.234,56 €`, Nachkommastellen und Symbol nach ISO 4217),
  Monate ausgeschrieben (`März 2026`), Anteile in Prozent mit zwei Stellen; „—" als Währung bei leerer
  Historie. Wiederkehr nach Jahresbetrag absteigend; „Platz im Ranking" im Ranking vorn, bei den
  Spar-Kandidaten hinten. Der Bericht entsteht vor dem Kategorie-Export: scheitert er, gibt es keinen
  Export. Eine
  eigenständige Seite: CSS inline, keine externe Ressource, kein JavaScript.

**Belegt durch** (`crates/io/src/report/tests.rs` aus reinen Fixture-DTOs, `crates/io/src/run/bericht_tests.rs`
über echte Läufe, `crates/auswerten_specs/tests/aderlass.rs`, `crates/obfuskierung_specs/tests/personen_namen.rs`):

- AK 1: `der_bericht_entsteht_aus_reinen_fixture_dtos_mit_allen_abschnitten`,
  `der_bericht_zeigt_die_errechneten_werte_in_euro_monaten_und_prozent`,
  `eine_jaehrliche_wiederkehr_steht_als_jaehrlich_und_die_teuerste_zuerst`; dazu in den Specs
  `die_umbuchungen_stehen_je_monat_und_gegenkonto_rolle_ueber_alle_fenster_des_laufs`,
  `die_gebuehren_stehen_je_monat_und_belastetem_konto_ueber_alle_fenster_des_laufs`.
- AK 2: `ohne_eine_einzige_kategorie_steht_jeder_treiber_als_unbekannt_da`,
  `eine_leere_historie_ergibt_einen_bericht_ohne_eintraege_und_ohne_waehrung`.
- AK 3: `die_kategorie_aendert_keine_zahl_sie_ist_nur_ein_attribut_des_treibers`.
- AK 4: alle Tests in `report/tests.rs` — kein SQLite, kein Kategorisierer.
- AK 5: `fehlen_der_analyse_buchungen_nennt_der_bericht_ihre_zahl_ganz_oben`,
  `ein_lauf_mit_fehlenden_buchungen_nennt_ihre_zahl_im_bericht`.
- AK 6: `ein_etikett_mit_markup_erscheint_escaped`,
  `die_kategorie_kommt_aus_dem_register_und_erscheint_escaped`.
- Datenschutz: `ein_echter_lauf_schreibt_einen_bericht_ohne_iban_bic_klarname_token_und_seed` (liest
  IBANs, BICs und Personen-Namen aus der Lauf-CSV selbst), `konten_erscheinen_ueber_ihre_rolle_nie_als_token`,
  `ein_firmenname_der_eine_person_der_namensliste_nennt_erscheint_nur_als_glaeubiger_id`,
  `die_beschriftung_kommt_aus_dem_anker_nicht_aus_dem_schluessel`,
  `ein_bereits_gescrubbter_firmenname_erscheint_nur_als_glaeubiger_id`,
  `zwei_pseudonyme_mit_gleichem_endstueck_werden_laenger_beschriftet`. Den Anker selbst halten die
  `auswerten_specs` fest (jeder erwartete Treiber trägt ihn, darunter Händler hinter einem Durchleiter
  und Personen).
- ADR-0016: `der_bericht_zeigt_genau_den_eigenen_lauf_auch_neben_einem_frueheren`,
  `der_bericht_liest_die_eigene_run_id_auch_wenn_ein_juengerer_fremder_lauf_in_der_datei_liegt`,
  `ein_gescheiterter_lauf_schreibt_keinen_bericht_und_laesst_den_alten_stehen`.

**Grenzen.**

- **Eine natürliche Person mit Gläubiger-ID**, die **nicht** in `personen.toml` steht, erscheint mit
  ihrem Namen — sie ist für die Analyse eine Firma (die Gläubiger-ID ist das Signal), und nur die
  Namensliste sagt etwas anderes. Abhilfe: den Namen in `personen.toml` eintragen.
- Der Firmenname ist so gut gescrubbt wie aller Freitext — heuristisch, nicht beweisbar vollständig.
