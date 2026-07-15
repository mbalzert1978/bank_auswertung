# Zeitfenster-Collector als paginierender Analyse-Kopf, zwei Analyse-Reichweiten

> **Status: Entwurf (entschiedener Plan, noch nicht gebaut).** Verfeinert
> [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) (fügt der Pipeline einen Schritt
> `sammeln` zwischen `obfuskieren` und `auswerten` hinzu) und stützt sich auf
> [ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) (reine Pipeline, Outbox-Mechanik
> in `io`), [ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) (durchweg
> terminal, uniformes ID-Handoff) und [ADR-0010](0010-seed-basierte-pseudonym-stabilitaet.md)
> (seed-basierte Token). Die Pipeline-Garantien (atomar, at-least-once, wiederaufsetzbar) und die
> Datenschutz-Grenze an `obfuskieren` bleiben unberührt.

## Problem

`auswerten` ist im Kern **N→1**: Wiederkehr-Erkennung, Pareto-Ranking und Cashflow sind
**Reduktionen über eine Menge** von Buchungen — eine einzelne `Obfuscated`-Nachricht kann man nicht
sinnvoll auswerten. Die Pipeline stromaufwärts ist dagegen pro-Buchung (`obfuskieren` ist 1:1). Es
fehlt (a) eine **Barriere** „alle relevanten Buchungen liegen vor → jetzt rechnen" und (b) eine Art,
die Analyse über eine potenziell **große Historie** zu **begrenzen und zu paginieren**.

Zweitens hat die Analyse **zwei Reichweiten**, die sich widersprechen, wenn man sie in denselben
Fenster-Schnitt presst: **periodische** Auswertungen (Cashflow/Pareto je Zeitraum) wollen einen
begrenzten Zeit-Slice; **Wiederkehr** ist per Definition **fenster-übergreifend** (ein Monats-Abo
erscheint einmal je ~30-Tage-Slice — isolierte Slices machen es unsichtbar).

## Entscheidung A — neuer Pipeline-Schritt `sammeln` (Collector)

```
einlesen → obfuskieren → sammeln → auswerten → report
 1→N          1:1         Pager       Fenster
(Trigger)   (pro Buchung)          (pro Fenster + Historie)
```

`sammeln` ist eine eigene Slice-Triade und ein Outbox-Consumer wie die anderen — terminal, uniformes
ID-Handoff. Er **sammelt keine Daten** (die obfuskierten Zeilen liegen bereits in der DB); seine eine
Aufgabe ist, das **Auswertungsfenster** über den Buchungsdaten zu **definieren** und dem Analyzer den
Datumsbereich zu übergeben. Der bisher gedachte „auswerten-Slice" **spaltet** damit in zwei Triaden:
`sammeln` (Fenster-Definition) + `auswerten` (Analyse).

## Entscheidung B — `sammeln` ist ein trigger-getriebener, sich selbst fortsetzender Pager

- **Zwei Kommandos, zwei Handler** (statt eines Handlers mit `Option<anchor>`):
  - **Anstoß (feldlos) — `StartAnalysis`:** ein reiner Auslöser auf der Outbox, **derselbe
    Kickoff-Mechanismus wie `einlesen`** ([[einlesen-kickoff-trigger-ueber-outbox]], Variante A; Decoder
    immer `Some`). Dem Slice ist egal, **wer** ihn einreiht — **heute manuell** (`main`/CLI „auswerten"),
    **später** io's Broker beim Leerlaufen des Imports. Der **Start-Handler** liest **einmalig**
    `max(booking_day)` aus der DB (Port D) und publiziert `AdvanceWindow{anchor: max}`.
  - **Fenster-Schritt — `AdvanceWindow{anchor}`:** trägt **immer** einen Anker (nie optional). Der
    **Window-Handler** rechnet die Seite `[anchor − days + 1 … anchor]`, legt die Fenster-Definition ab
    (DB vergibt `window_id`) und macht in **derselben** Transaktion **zwei Publishes** über **zwei
    Domänen-Sink-Ports**:
    1. **downstream → `auswerten`:** `FensterGeschlossen{window_id}` — der Zeiger auf die **abgelegte
       Fenster-Definition** (die ID der collected Daten, die der Analyzer zum Lesen braucht; Tabelle =
       Wahrheit).
    2. **self-continue → `sammeln`:** `AdvanceWindow{anchor: from − 1}`, **nur falls** ältere Daten
       existieren (`has_bookings_before(from)`) — sonst endet die Kette (erschöpft).

    Store + **beide** Publishes committen **atomar** zusammen oder gar nicht. Die Unit-of-Work trägt das
    bereits: `einlesen` publiziert schon **N** Nachrichten je Lauf in **einer** Unit — mehrere Publishes
    je Transaktion sind gelöst; neu ist nur, dass die zwei **verschiedene** `message_type`s haben (der
    generische `publish<T: OutboxCommand>` nimmt heterogene Typen).
- **Fenster = Seitengröße des Pagers** (nicht der Berichtszeitraum): `[anchor − window_days + 1 …
  anchor]`. `window_days` kommt aus der **AppConfig** (`collector_window_days`, real **~400** — bewusst
  **> 365**, damit ein voller Jahreszyklus in **eine** Seite fällt); **io liest die Config** und reicht
  sie dem Handler als Wert herein — genau wie den `seed` bei `obfuskierung` (die Domäne liest keine
  Datei). Cashflow wird **innerhalb** der Seite pro Monat gebucketet.
- **Warum die Zwei-Kommando-Form eleganter ist (ADR-0005):** kein `Option<anchor>`, dessen `None`
  „lies aus der DB" bedeutet — der **Typ** trägt die Absicht (feldloser `StartAnalysis` startet,
  `AdvanceWindow` trägt garantiert einen Anker). Der illegale Zustand „leerer Anker" ist
  **unkonstruierbar**, nicht „geprüft". Jeder Handler hat **eine** Aufgabe; zwei `message_type`s, zwei
  Consumer im selben Slice, vom Dispatcher uniform aufgelöst.
- **Anker im Transport:** `AdvanceWindow` trägt seinen Tag-Stamp als stringifizierten `partition_key`,
  **kein serde** — wie `obfuskieren` die id trägt.
- **Terminierung** garantiert: `from` sinkt je Schritt; die Kette endet, sobald keine älteren Daten mehr
  da sind (erschöpft). Kein Zähler, kein Watermark-Sonderfall.
- **Dimensionierung (real):** Input ~3 Jahre (≈ 1095 Tage), Seite ~400 Tage → **~3 Seiten**. Der Pager
  feuert wenige Male, dann erschöpft — die Kosten liegen nicht in der Seitenzahl.

> **Verworfen: per-Insert-Trigger** (Collector konsumiert jede `Obfuscated`, prüft „genug Rows → go").
> Zwei Gründe: (1) N Queries; (2) bei einem **Zeit**fenster ist die Abschlussbedingung ein
> Watermark „eine Buchung datiert **nach** dem Fensterende traf ein" — ein **endlicher** Bank-Export
> liefert das für das letzte Fenster **nie**, es bräuchte trotzdem einen expliziten Flush. Per-Insert
> kostet mehr **und** löst das Problem nicht.

## Entscheidung C — Buchungsdatum als sortierbarer Tag-Stamp in der DB

Jedes Datumsfenster braucht auf der obfuskiert-/Analyse-Seite ein **sortier-/abfragbares** Datum. Heute
dekodiert `einlesen` das Datum strukturiert (`Line.booking_day: CalendarDay`), an der obfuskieren-Grenze
wird es aber wieder zu einem **opaken String** plattgedrückt (`IngestedRow.booking_day: String` →
`PassThrough: NonEmptyString`). Festlegung: das Datum wird als **tag-genauer Integer-Stamp** persistiert
(Mitternacht UTC bzw. „Tage seit Epoche") — numerisch sortierbar, `MAX()`/Range-Query trivial,
Fenster-Arithmetik `± days`. Feste Zone (UTC) hält es deterministisch. Der Stamp ist die Query-Achse;
`auswerten` rekonstruiert für die Domänensicht (Monats-Bucketing) wieder `CalendarDay`. **io-Arbeitspaket.**

## Entscheidung D — Anker hinter einem Port (`WindowAnchor`)

Der Handler nimmt `anchor: CalendarDay` aus einem Port und weiß **nicht**, ob das `today` oder `max` ist;
die TestApi fakt ihn mit festem Datum. io verdrahtet die Implementierung:

- **`max(booking_day)`** (empfohlen): `SELECT MAX(<stamp>)` — eine indizierte Query, **datenrelativ**.
  Richtig für **historische Exporte** (ein `today − 30d`-Fenster liefert bei alten Daten nichts) und
  **deterministisch** (gleiche DB → gleiches Fenster, testbar ohne Fake-Uhr).
- **`today`** (Alternative): ein `Clock`-Port. Nachrüstbar, weil beide hinter demselben Port sitzen.

## Entscheidung E — `auswerten` bedient zwei Reichweiten (Fold-Log für Wiederkehr)

- **Pro-Fenster** (aus `FensterGeschlossen`): Cashflow je Zeitraum, Gebühren-Klassifikation, Pareto
  **innerhalb** des Zeitraums.
- **Über die Historie (Fold-Log):** jeder Fenster-Lauf **hängt** je Buchung `(Fingerabdruck → (Tag,
  Betrag))` an ein **Vorkommens-Log** an. Die **Wiederkehr-Erkennung** liest das **akkumulierte** Log
  über die ganze Zeitachse — eine **kompakte Projektion**, nicht die rohen Buchungen der ganzen
  Historie. So bleibt der Pager der Motor und die Wiederkehr sieht trotzdem alle Perioden.
- **Warum das Log auch bei 400-Tage-Seiten nötig ist:** eine **~400-Tage-Seite** enthält **monatliche**
  Wiederkehr (~13 Vorkommen) vollständig, aber ein **jährliches** Abo erscheint darin nur 1× (an den
  Rändern evtl. 2×, unzuverlässig). „Jährlich wiederkehrend" (Issue 0005, Jahreskosten) braucht **2+
  Vorkommen im 12-Monats-Abstand** über die Seiten hinweg → nur das akkumulierte Log belegt es.

## Entscheidung F — was `auswerten` sonst besitzt (aus 0004–0011 übernommen)

- **Fingerabdruck** als reine Domänenfunktion (inkl. Zahlungsintermediär-Erkennung + Händler-Token),
  [ADR-0004](0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)/[0006](0006-pseudonym-stabilitaet.md).
- **`Gegenkonto::Eigen|Fremd` auf Token** — Gegenkonto-`KONTO_x` gegen die mit **demselben Seed**
  tokenisierte Menge der eigenen Konten; braucht die **Eigene-Konten-Config** (Issue 0002, jetzt
  token-orientiert, IBAN + Rolle).
- **`Kategorisierer`** nur für der Domäne **unbekannte** Fingerabdrücke (LLM-Orakel, Export→Chat→Import);
  Egress-DTO `KategorieAnfrage` trägt die `assert_not_impl_any!`-„kein `Serialize`"-Sperre.

## Konsequenzen / Vorbedingungen

- **io:** (1) sortierbarer Tag-Stamp in der obfuskiert-Ablage (Entscheidung C); (2) wer `StartAnalysis`
  einreiht (heute manuell; später Broker-Drain).
- **Leeres Fenster / leere DB:** terminaler „nichts auszuwerten"-Fehler. *(offen — bestätigen; Alternative:
  valider leerer Report.)*
- Die Fach-Issues **0004** (Umbuchung-Netting), **0005** (Wiederkehr), **0006** (Gebühren), **0007**
  (Pareto/Sparkandidaten) hängen sich als **Analyzer-Bausteine** unter `auswerten`; **0002** wird ein
  `auswerten`-Input.

## Bewusst vertagt

Das echte `io` (Broker-Auto-Trigger beim Drain, `sqlx`-Stores, Tag-Stamp-Persistenz), der `report`,
persistente Seed-Verwaltung und die `ObfuskierterText`-Scrubbing-Regeln.

## Offene Sprach-Begriffe (zu grillen)

`Auswertungsfenster`, Slice `sammeln`/Collector, Trigger `StartAnalysis`, Fenster-Schritt
`AdvanceWindow`, Ausgangs-Nachricht `FensterGeschlossen`, `Vorkommens-Log`, Port `WindowAnchor` — vor dem
Bau gegen CONTEXT.md schärfen.
