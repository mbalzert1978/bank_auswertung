---
id: "0005"
title: "Wiederkehr-Erkennung"
status: closed
---

# Wiederkehr-Erkennung

**Typ:** AFK

> GitHub-Issue: [#94](https://github.com/mbalzert1978/bank_auswertung/issues/94)

> **Verfeinert durch [Issue 0007](0007-aderlass-pareto-ranking-und-spar-kandidaten.md):**
> - Die Wiederkehr gruppiert die genetteten Abflüsse nicht mehr je **Fingerabdruck**, sondern je
>   **Kostentreiber** (`CostDriver`: Buchungsart über ihren Fingerabdruck, die Gebühren eines Kontos,
>   oder der Sammel-Treiber ohne Gegenpartei, der keine Wiederkehr bekommt) — **eine** Gruppierung,
>   die sich Wiederkehr und Pareto-Ranking teilen.
> - `RecurrenceRow` trägt darum `driver: CostDriverRow` statt `fingerprint`.
> - **Gebühren können wiederkehren:** eine monatliche Kontoführung ist ein wiederkehrender Abfluss
>   mit vollem Jahresbetrag (Treiber `Fee(konto)`).

> **Verortung ([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).**
> Analyzer-Baustein **unter dem `auswerten`-Slice**, die **fenster-übergreifende** Reichweite. Die
> Wiederkehr läuft als **Read-Port-Query über alle `Auswertungsfenster`-Aggregate** eines Laufs,
> ausgelöst durch `HistorieVollständig` (Abschluss des Pagers `fenstern`) — **kein** Fold-Log /
> `Vorkommens-Log` (das im Entwurf gedachte akkumulierende Log ist **verworfen**, #30). Nur die Sicht
> über **alle** Fenster belegt eine **jährliche** Wiederkehr (2+ Vorkommen im 12-Monats-Abstand); ein
> jährliches Abo erscheint je ~400-Tage-Seite nur 1×.

## Was zu bauen ist

Zweiter Teil des Analyse-Kerns: deterministische Erkennung wiederkehrender
Abflüsse (Abos, Daueraufträge) — inkl. Jahreskosten (jährlich statt monatlich
abgebuchte Abos) und Preissteigerungen bei ansonsten gleichbleibenden
wiederkehrenden Abflüssen.

Vertieft hier `Fingerabdruck` (D2) um die Zahlungsintermediär-Erkennung:
PayPal/Klarna/Amazon Payments etc. dürfen nicht alle dahinterliegenden
Händler zu einem Klumpen zusammenfassen — die Wiederkehr muss am echten
Händler-Token hängen, nicht am Durchleiter. Das ist Domänenwissen (nicht
Adapter-Konzern) und wirkt sich auch auf die spätere Kategorisierung aus
(Textwahl Verwendungszweck vs. Empfänger bei Intermediären, siehe Slice 0011).

Arbeitet auf bereits genetteten Abflüssen (Slice 0004) — eine wiederkehrende
Umbuchung an ein eigenes Konto zählt nicht als wiederkehrender Kosten-Treiber.

## Abnahmekriterien

- [x] Eine Serie gleichartiger monatlicher Ausgangs-Buchungen (gleicher
      Fingerabdruck, ähnlicher Betrag, regelmäßiger Abstand) wird als
      wiederkehrender Abfluss erkannt.
- [x] Eine jährlich abgebuchte Buchung (12-Monats-Abstand) wird ebenfalls als
      wiederkehrend erkannt, nicht übersehen.
- [x] Eine Preissteigerung bei sonst gleichbleibendem wiederkehrendem Abfluss
      (gleicher Fingerabdruck, gestiegener Betrag ab einem Stichtag) wird
      erkannt und ausgewiesen.
- [x] Zwei verschiedene Händler, die beide über denselben
      Zahlungsintermediär abgerechnet werden, ergeben **unterschiedliche**
      Fingerabdrücke/Wiederkehr-Gruppen — Test mit mindestens zwei
      Fixture-Händlern hinter demselben Intermediär.
- [x] Eine wiederkehrende Umbuchung an ein eigenes Konto erscheint **nicht**
      in der Wiederkehr-Liste der Kosten-Treiber.
- [x] Test deckt den DTO-Vertrag ab: Buchungs-Liste rein, Wiederkehr-Ergebnis
      raus, ohne Datei-/Netzzugriff.

## Wie es gebaut wurde

**Ort.** Reine Domänen-Logik in `auswerten_domain`, aufgesetzt auf die fenster-übergreifende
Reichweite (ADR-0013, Entscheidung E). Der `HistoryEvaluationHandler` liest wie bisher **eine**
Query über alle Fenster des Laufs und bildet daraus die `HistoryEvaluation`: die Vorkommen der
Zeitachse (unverändert, ungefiltert) **und** die erkannten `Recurrence`s. Er legt weiterhin **nichts**
ab — das Ergebnis ist sein Rückgabewert.

**Ablauf je Lauf** (`HistoryEvaluation::of`, `Recurrence::detect`):

1. **Nur Abflüsse** — ein regelmäßiger Zufluss (Gehalt) ist kein Kosten-Treiber.
2. **Genettet** — eine Buchung, deren Gegenkonto-Token in der Eigene-Konten-Config steht, fällt
   heraus. Dafür gibt es **eine** Regel (`WindowedBooking::transfer_role`), die jetzt beide
   Reichweiten benutzen: was im Cashflow als Umbuchung fehlt, kann nicht als Abo auftauchen. Weil
   dieser Vergleich auf Token läuft, prüft auch diese Reichweite den `SeedMarker`
   (`EvaluationError::SeedMismatch`); eine **leere** Lauf-Spanne braucht keinen Abgleich. Die
   Prüfung ist _parse, don't validate_: `OwnAccounts::verified_for(&SeedMarker)` liefert eine
   `VerifiedOwnAccounts`, und nur sie kann ein Gegenkonto einordnen — beide Reichweiten teilen das.
3. **Je Fingerabdruck gruppiert** — bei einem Durchleiter steckt der Händler-Token schon im
   Fingerabdruck (0041), zwei Händler hinter PayPal sind also zwei Gruppen. Eine Buchung ohne
   Gegenpartei hat keinen Fingerabdruck und gehört zu keiner Gruppe.
4. **Chronologisch geordnet per Typ:** die Abbuchungen einer Art bilden eine `ChargeSeries`
   (sortiert und nicht-leer per Konstruktion); Takt, Preisstufen und jüngstes Konto lesen nur sie.
5. **Takt prüfen**, dann **Preisstufen bilden**, dann das Ergebnis ableiten.

**Festlegungen** (benannte Konstanten in `value_objects/rhythm.rs` und `recurrence.rs`):

| Festlegung | Wert | Warum |
|---|---|---|
| Monatlicher Abstand | 25–35 Tage | Monat = 28–31 Tage, plus ±3 Bankarbeitstage Verschiebung über Wochenende/Feiertag. Weit weg von Woche und Quartal. |
| Jährlicher Abstand | 350–380 Tage | 365/366 Tage plus Verschiebung; zwischen Halbjahr und Jahr liegt nichts, das hineinfiele. |
| Mindestzahl monatlich | 3 Abbuchungen | Zwei Käufe im Monatsabstand sind schnell Zufall; die dritte auf demselben Takt ist ein Muster. |
| Mindestzahl jährlich | 2 Abbuchungen | Mehr geht nicht: ~3 Jahre Historie zeigen ein Jahresabo zwei- bis dreimal (ADR-0013: „2+ Vorkommen im 12-Monats-Abstand"). |
| Takt-Strenge | **jeder** Abstand im selben Band | Ein ausgelassener Monat (~60 Tage) oder eine Doppelbuchung (0 Tage) bricht die Reihe. Lieber ein Abo nicht erkennen als einen Rhythmus behaupten. |
| „Ähnlicher Betrag" | ±5 % zum ersten Betrag der Preisstufe | Abos/Daueraufträge buchen centgenau; 5 % fängt nur Rundungs-/Kursreste. Größer darf es nicht sein, sonst verschluckte die Ähnlichkeit übliche Preiserhöhungen (12,99 → 13,99 = +7,7 %). |
| Preissteigerung | ein Betrag **über** der Toleranz beginnt eine neue, höhere Preisstufe; ausgewiesen wird die **jüngste** Stufung (Stichtag = erste Abbuchung der neuen Stufe, alt/neu = typischer Betrag der beiden Stufen) | Die jüngste Stufung erklärt den heutigen Preis. |
| Betrag sinkt | Reihe ist **keine** Wiederkehr — sowohl ein Betrag unter der laufenden Stufe als auch eine neue Stufe, deren **typischer** Betrag nicht über dem der vorigen liegt | Was mal steigt, mal fällt, ist kein gleichbleibender Abfluss. Die zweite Bedingung ist nötig, weil der Schwellwert am ersten Betrag einer Stufe misst, ausgewiesen aber der typische wird (100 / 105 / 105 / 106 / 101 / 101 wäre sonst eine „Steigerung" von 105 auf 101). `PriceIncrease::of` gibt zudem `None`, wenn `after > before` nicht gilt — eine Senkung als Steigerung ist unkonstruierbar. |
| Typischer Betrag | unterer Median der **aktuellen** Preisstufe | Immer ein wirklich abgebuchter Betrag, keine Rundung, robust gegen Ausreißer. |
| Jahresbetrag | typischer Betrag × 12 (monatlich) bzw. × 1 (jährlich) | Rechnet mit dem **aktuellen** Preis — das ist, was künftig abgeht, und die Bezugsgröße für Spar-Kandidaten (0007). |
| Belastetes Konto | Auftragskonto der **jüngsten** Abbuchung | Dort wird heute abgebucht, dort setzt eine Kündigung an. Gruppiert wird nur nach Fingerabdruck, damit ein Konto-Umzug die Reihe nicht zerreißt. |

**Ausgabe (öffentliche Adapter-Fläche).** `HistoryEvaluationHandler::new(&dyn OccurrenceSeam,
OwnAccountsConfig)` + `Handler::handle(EvaluateHistoryCommand)` → `HistoryEvaluationRow {
occurrences: Vec<OccurrenceRow>, recurrences: Vec<RecurrenceRow> }`. Das Kommando entsteht über
`EvaluateHistoryCommandDecoder::decode("{from}:{to}", "")`. `RecurrenceRow { fingerprint, rhythm:
RecurrenceRhythm::{Monthly,Yearly}, typical_amount_minor_units, annual_amount_minor_units,
price_increase: Option<PriceIncreaseRow { since_day, before_minor_units, after_minor_units }>,
charged_account }`, nach Fingerabdruck geordnet. Kein Ranking — das ist 0007. Datenschutz: nur
Fingerabdruck, Beträge, Tag-Stamp und Konto-Token; `HistoryEvaluationRow`, `RecurrenceRow`,
`PriceIncreaseRow` und `OccurrenceRow` tragen den `assert_not_impl_any!(…: Serialize)`-Guard
(Egress-Kandidaten für Bericht 0012 und Chat-Export 0011).

**io (minimal).** Der Lese-Seam heißt jetzt `OccurrenceSeam::history_in` und liefert eine
`RunHistoryRow { seed_marker, bookings }`; io liest den Marker wie beim Einzel-Fenster aus den Zeilen
und failt terminal, wenn die Spanne Buchungen aus verschiedenen Tokenisierungen trägt.
`history_consumer` bekommt die Eigene-Konten-Config, der `HistoryEvaluationSink` die ganze
`HistoryEvaluationRow` (der reale Lauf zählt weiter nur die Vorkommen).

**Tests** (`crates/auswerten_specs/tests/wiederkehr.rs`, je Kriterium):

- monatlich: `eine_monatliche_serie_gleicher_ausgaenge_ist_ein_wiederkehrender_abfluss`
  (+ `ein_leicht_schwankender_betrag_ist_dieselbe_wiederkehr_ohne_preissteigerung`,
  `zwei_monatliche_abbuchungen_sind_noch_keine_wiederkehr`, `ein_unregelmaessiger_abstand_ist_keine_wiederkehr`)
- jährlich: `eine_jaehrliche_abbuchung_im_zwoelf_monats_abstand_ist_wiederkehrend`
  (+ `ein_jahresabo_mit_verschobenem_abbuchungstag_bleibt_jaehrlich_und_zeigt_seine_preissteigerung`)
- Preissteigerung: `eine_preissteigerung_ab_einem_stichtag_wird_erkannt_und_ausgewiesen`
  (+ `ein_gesunkener_betrag_bricht_die_serie`,
  `eine_stufe_mit_gesunkenem_typischem_betrag_ist_keine_preissteigerung`)
- Durchleiter: `zwei_haendler_hinter_demselben_durchleiter_ergeben_zwei_wiederkehr_gruppen`
- Umbuchung: `eine_wiederkehrende_umbuchung_auf_ein_eigenes_konto_erscheint_nicht`
  (+ Gegenprobe `derselbe_dauerauftrag_an_ein_fremdes_konto_ist_ein_wiederkehrender_abfluss`,
  `ein_regelmaessiger_zufluss_ist_kein_kosten_treiber`,
  `eine_eigene_konten_config_aus_anderer_tokenisierung_endet_terminal`,
  `eine_historie_ohne_seed_marker_endet_terminal_und_redigiert`,
  `eine_leere_lauf_spanne_braucht_keinen_seed_abgleich`)
- Grenze fixiert: `ein_einzelkauf_beim_selben_glaeubiger_bricht_die_reihe`
- io gegen echtes SQLite: `history_in_ueber_fenster_mit_verschiedenen_seeds_failt_terminal`,
  `history_in_ueber_fenster_einer_tokenisierung_traegt_deren_marker`
- DTO-Vertrag: `der_dto_vertrag_nimmt_buchungen_und_gibt_das_wiederkehr_ergebnis_ohne_datei_und_netz`

**Bewusste Grenzen** (heuristisch, nicht beweisbar vollständig):

- Schwankende Rechnungen (Strom, Mobilfunk mit Zusatzverbrauch) werden nicht erkannt — ein Betrag,
  der wieder sinkt, bricht die Reihe. Das Ticket zielt auf Abos/Daueraufträge.
- Eine Preiserhöhung unter 5 % geht in der Preisstufe auf und wird nicht ausgewiesen.
- Ein ausgelassener Monat bricht eine Monatsreihe.
- **Eine Art, eine Reihe:** gruppiert wird allein nach Fingerabdruck. Ein Einzelkauf beim selben
  Gläubiger wie ein Abo (gleiche Gläubiger-ID, kein Durchleiter) landet in derselben Reihe und
  zerreißt deren Takt — das Abo wird dann nicht ausgewiesen. Bewusst kein Sub-Clustering (etwa nach
  Betrag); festgehalten durch `ein_einzelkauf_beim_selben_glaeubiger_bricht_die_reihe`.
- Regelmäßige Bank-Entgelte ohne Gegenpartei haben keinen Fingerabdruck; sie weist die
  Gebühren-Sicht (0006) aus.
- Das DTO trägt keine Währung; die Beträge sind Minor Units der einen Währung der Fenster.
- **Wechselwirkung mit 0084:** ein wiederholter Lauf auf derselben DB verdoppelt heute die Buchungen
  im Lauf (siehe CLAUDE.md). Doppelte Abbuchungen am selben Tag ergeben den Abstand 0 und brechen
  jede Reihe — die Wiederkehr verschwindet dann, statt falsch zu zählen. Bis zur De-Duplizierung
  gilt: frische `--db` je Lauf.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
- Blockiert durch [0041](0041-fingerabdruck-als-domaenenfunktion.md) — die Wiederkehr schlüsselt
  auf den Fingerabdruck.
- Blockiert durch [0043](0043-historie-vollstaendig-handler-und-fenster-uebergreifende-query.md) —
  die Rhythmus-Logik läuft auf der fenster-übergreifenden Read-Port-Query.
