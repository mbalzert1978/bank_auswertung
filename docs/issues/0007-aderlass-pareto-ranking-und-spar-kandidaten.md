---
id: "0007"
title: "Aderlass, Pareto-Ranking und Spar-Kandidaten"
status: open
---

# Aderlass, Pareto-Ranking und Spar-Kandidaten

**Typ:** AFK

> GitHub-Issue: [#95](https://github.com/mbalzert1978/bank_auswertung/issues/95)

> **Verortung ([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).**
> Analyzer-Baustein **unter dem `auswerten`-Slice**, gebaut in der **fenster-übergreifenden**
> Reichweite (Handler auf `HistorieVollständig`, Ergebnis als Rückgabewert, nichts abgelegt) — nicht,
> wie ursprünglich geplant, pro Fenster. Begründung und Details: Abschnitt
> [„Wie es gebaut wurde"](#wie-es-gebaut-wurde).

## Was zu bauen ist

Vierter und letzter Teil des Analyse-Kerns, die drei eng gekoppelten
„Aderlass/Ranking/Sparen"-Fähigkeiten in einem Slice: der Aderlass/Cashflow je
Konto und Monat, ein Pareto-Ranking der größten Abflüsse darüber, und
strukturell gefundene Spar-Kandidaten, die direkt aus dem Ranking abgeleitet
werden (deshalb zusammen — Spar-Kandidaten sind kein eigenständiger
Berechnungsschritt, sondern eine Sicht auf die Pareto-Spitze).

Rechnet auf bereits genetteten Abflüssen (Slice 0004) plus den klassifizierten
Treibern aus Wiederkehr (Slice 0005) und Gebühren (Slice 0006) — das Ranking
soll wiederkehrende Kosten und Gebühren als Kandidaten mit einbeziehen, nicht
nur Einzelbuchungen.

## Abnahmekriterien

- [x] Cashflow/Aderlass wird je eigenem Konto und je Monat berechnet, über den
      gesamten aus der CSV verfügbaren Zeitraum.
- [x] Ein Pareto-Ranking sortiert die größten Abflüsse (inkl. wiederkehrender
      Treiber aus Slice 0005 und Gebühren aus Slice 0006) nach Anteil an der
      Gesamtsumme.
- [x] Spar-Kandidaten werden strukturell aus der Pareto-Spitze abgeleitet
      (z. B. teuerste wiederkehrende Treiber, größte Preissteigerungen) —
      nicht durch das LLM vorgeschlagen.
- [x] Test: eine genettete Umbuchung taucht **nicht** im Cashflow oder im
      Pareto-Ranking als Abfluss auf.
- [x] Test deckt den DTO-Vertrag ab: Buchungs-/Wiederkehr-/Gebühren-Ergebnisse
      rein, Cashflow- + Ranking- + Spar-Kandidaten-DTOs raus, ohne
      Datei-/Netzzugriff.

## Wie es gebaut wurde

**Ort: die fenster-übergreifende Reichweite, nicht die pro-Fenster.** Die Verortung oben (Pareto
innerhalb eines Fensters) ist überholt: ein Fenster ist eine ~400-Tage-*Seite* des Pagers, kein
Berichtszeitraum. Ein Ranking je Seite rankte nur einen Ausschnitt, ein Monat an der Seitengrenze
zerfiele in zwei halbe Buckets, und der Bericht (0012) müsste Rankings zusammenführen und Anteile neu
rechnen — Domänenwissen im Bericht. Der `HistoryEvaluationHandler` sieht dagegen **alle** Buchungen
**eines** Laufs (ADR-0016) und gibt das Ergebnis als Teil der `HistoryEvaluationRow` **zurück** —
nichts wird abgelegt (ADR-0013), **keine Schema-Änderung**. Der Pareto-Top-N speist den
Wiederkehr-Filter bewusst **nicht**: die Wiederkehr läuft ohnehin über alle Arten, ein Filter spart
nichts und verlöre Abos unterhalb der Spitze.

**Domäne** (`auswerten_domain`, alles in `HistoryEvaluation::of`):

- **Cashflow je Konto und Monat** (`AccountCashflow`): genettet (Umbuchungen raus, beide Richtungen),
  Gebühren drin; Schlüssel = belastetes Konto (`KONTO_…`-Token) + Monat, geordnet nach Konto, darin
  chronologisch. Die **Rolle** kommt aus der Eigene-Konten-Config und ist `None`, wenn die Config das
  Konto nicht führt.
- **Kostentreiber** (`CostDriver`, Summentyp): `Kind(Fingerabdruck)` | `Fee(belastetes Konto)` |
  `Unattributed`. Eine Gebühr (Buchungstext, 0006) geht **vor** dem Fingerabdruck.
- **Eine Gruppierung** (Review-Runde 1): die genetteten Abflüsse werden **einmal** nach `CostDriver`
  gruppiert (`DriverGroups`); daraus kommen die Wiederkehr je bestimmtem Treiber — **auch für
  `Fee`**: eine monatliche Kontoführung ist wiederkehrend und trägt ihren vollen Jahresbetrag;
  `Unattributed` bekommt keine — und die Summe je Treiber fürs Ranking. Zwei Gruppierungen
  (Fingerabdruck für die Wiederkehr, Treiber fürs Ranking) ließen eine einzelne Entgeltzeile mit
  Gegenpartei die Abo-Reihe des Händlers zerreißen. `RecurrenceRow` trägt darum den Treiber
  (`driver: CostDriverRow`) statt eines Fingerabdrucks.
- **Grenze (Review-Runde 2, W5):** `Fee(konto)` fasst **alle** Gebühren eines Kontos zu einer Reihe
  zusammen. Eine zweite Gebührenart auf demselben Konto (z. B. ein Auslandseinsatz-Entgelt neben der
  Kontoführung) bricht den Takt — die Kontoführung gilt dann nicht als wiederkehrend, ihr
  Jahresbetrag ist der Durchschnitt je Jahr. Festgehalten in der Spec
  `eine_zweite_gebuehrenart_auf_demselben_konto_bricht_den_takt_der_kontofuehrung`. Ein feinerer
  Schnitt (je Gebührenart) ist bewusst nicht gebaut; das entscheidet Markus beim HITL zu 0006.
- **Pipeline** in `HistoryEvaluation::of`: `netted_outflows → DriverGroups → {recurrences, rank} →
  savings_candidates`, daneben `account_cashflows` und `occurrences`; die genetteten Abflüsse werden
  einmal durchlaufen.
- **Eine Währung je Lauf** (Review-Runde 1): diese Reichweite summiert über alle Fenster. Der Lese-Seam
  liefert darum die Währungen der beteiligten Fenster (`RunHistoryRow::currencies`, io:
  `SELECT DISTINCT currency …`), die Domäne baut `RunHistory` nur über **genau eine**
  (`EvaluationError::MixedCurrencies`, terminal, ohne die Werte). Die `HistoryEvaluationRow` trägt die
  eine `currency` (für den Bericht). Abflüsse ohne
  Gegenpartei, die keine Gebühr sind (z. B. Barabhebung), werden **zu einem Sammel-Treiber
  zusammengefasst** statt weggelassen — sonst ergäben die Anteile nicht das Ganze und der Bericht
  verschwiege abgeflossenes Geld. Der Sammel-Treiber ist **nie** Spar-Kandidat (keine Art, auf die
  man handeln könnte).
- **Pareto-Ranking** (`RankedDriver`): genettete Abflüsse je Treiber summiert, absteigend nach
  Betrag, Gleichstand nach Treiber (deterministisch). Anteil und kumulierter Anteil als `Share` in
  **Basispunkten** (10 000 = 100 %, ganzzahlig, kein `f64`). Abgerundet wird **nur** der kumulierte
  Anteil (aus der kumulierten Summe); der Einzelanteil ist die Differenz zum Vorplatz — so ergeben die
  Einzelanteile **genau** 10 000 (getrennt gerundet waren es z. B. 9 998). Die Summe aller Treiber ist genau
  der Abfluss des genetteten Cashflows. Wiederkehrende Treiber (0005) tragen ihre `Recurrence` (Takt,
  Preissteigerung) im Ranking.
- **Jahresbetrag** (Bezugsgröße, vom Orchestrator bestätigt): wiederkehrend → aus der Wiederkehr
  (aktueller Preis × Abbuchungen/Jahr); alles andere (Einzelkauf, unregelmäßig, Gebühren,
  Sammel-Treiber) → **Durchschnitt je Jahr** über den beobachteten Zeitraum (erster bis letzter
  Buchungstag des Laufs, alle Konten): `Summe × 365 / max(Tage, 365)`. **Nie hochgerechnet:** bei
  weniger als einem Jahr Daten ist es die Summe — ein Einzelkauf in einem 3-Monats-Auszug wird nicht
  zu vier Käufen. Preis der Vorsicht: eine Monatsgebühr in einem kurzen Auszug erscheint mit weniger
  als ihrem vollen Jahr.
- **Spar-Kandidaten** (`SavingsCandidate`, Entscheidung V3): jeder Treiber außer dem Sammel-Treiber,
  dessen Jahresbetrag die `SavingsThreshold` **erreicht** (einschließlich). Die Schwelle ist
  `NonZeroU64` von der Domäne bis zum Loader (`SavingsThreshold`, `SavingsThresholdConfig`,
  `SparenConfig`) — eine Schwelle von 0 ist unkonstruierbar. Geordnet nach Jahresbetrag
  absteigend (nicht nach Ranking-Platz), mit `pareto_rank`, Anteil, Takt und **Preissteigerung** als
  Merkmal. Kein LLM.

**Config:** neue kleine Datei mit Pflicht-Option `--sparen` (`jahresbetrag_ab_cent = 10000`, positive
Ganzzahl in Cent), Loader `io::config::SparenConfig` nach dem Muster von `FensterConfig`, redigierte
Fehler (`SavingsThresholdMissing`/`SavingsThresholdInvalid`, `ConfigKind::Sparen`), Vorlage
`sparen.example.toml`, Lauf-Fixture `crates/io/fixtures/lauf/sparen.toml`. Eigene Datei statt Schlüssel
in der Fenster-Config: Pager-Mechanik und Nutzer-Vorliebe ändern sich aus verschiedenen Gründen. Nicht
gitignored — der Wert ist nicht vertraulich.

**Ausgabe** (für Bericht 0012 / Export 0011), öffentliche Adapter-DTOs, alle mit
`assert_not_impl_any!(…: Serialize, Deserialize)`: `HistoryEvaluationRow` trägt zusätzlich
`account_cashflows: Vec<AccountCashflowRow>`, `ranking: Vec<DriverRankRow>` (mit `CostDriverRow`) und
`savings_candidates: Vec<SavingsCandidateRow>` (`{ pareto_rank, ranked: DriverRankRow }` — eine
Abbildung, keine doppelten Felder), dazu `currency: Option<String>`. Inhalt nur Token, Fingerabdrücke, Beträge, Monate,
Tag-Stamps. Die Schwelle kommt als `SavingsThresholdConfig` herein
(`HistoryEvaluationHandler::new(seam, own_accounts, savings_threshold)`, `io::auswerten::history_consumer`
analog). Der Bericht bekommt die Zeile über den `HistoryEvaluationSink` des Laufs; heute zählt
`io::run::CountingHistorySink` nur Vorkommen und Spar-Kandidaten (`RunReport::savings_candidates`,
Protokollzeile „Spar-Kandidaten: n") — die Zeile für **seinen** Lauf festzuhalten ist Sache von 0012.

**Tests:** `crates/auswerten_specs/tests/aderlass.rs` (15 Specs über die `HistoryTestApi`, u. a.
genettete Umbuchung weder im Cashflow noch im Ranking, DTO-Vertrag ohne Datei/Netz, Summe der Anteile
= 10 000, monatliche Gebühr wiederkehrend, Gebühr mit Gegenpartei, gemischte Währungen),
`io::run::aderlass_tests` (zwei echte Läufe, echt geladene Config: die Konto-Rolle kommt im
`account_cashflows` an, Ranking- und Cashflow-Abfluss stimmen überein),
`io::config::sparen` (Loader gegen Text und Fixture), `io::run::tests` (fehlende Spar-Config,
Kandidaten im echten Lauf, Zahlen des wiederholten Laufs inkl. Kandidaten-Zahl gleich).

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0004](0004-umbuchung-netting.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md)
- Blockiert durch [0006](0006-gebuehren-klassifikation.md)
- Blockiert durch [0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md) — das Skelett der
  `auswerten`-Triade (gebaut wurde der Aderlass dann auf der fenster-übergreifenden Reichweite, siehe
  „Wie es gebaut wurde").
