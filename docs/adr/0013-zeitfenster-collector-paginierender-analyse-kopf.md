# Zeitfenster-Schritt `fenstern` als paginierender Analyse-Kopf, zwei Analyse-Reichweiten

> **Status: Akzeptiert und gebaut** (Kette 0036–0045 in `main`, gegen Fakes; das reale `io`
> — SQLite/`rusqlite` (synchron, [ADR-0014](0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)),
> Tag-Stamp-Persistenz, Config-Loader — bleibt bewusst der letzte Bauschritt).
> Verfeinert
> [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) (fügt der Pipeline einen Schritt
> `fenstern` zwischen `obfuskieren` und `auswerten` hinzu) und stützt sich auf
> [ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) (reine Pipeline, Outbox-Mechanik
> in `io`), [ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) (durchweg
> terminal, uniformes ID-Handoff) und [ADR-0010](0010-seed-basierte-pseudonym-stabilitaet.md)
> (seed-basierte Token). Die Pipeline-Garantien (atomar, at-least-once, wiederaufsetzbar) und die
> Datenschutz-Grenze an `obfuskieren` bleiben unberührt.
>
> **Herkunft:** druckgetestet über die Wayfinder-Karte
> [#27](https://github.com/mbalzert1978/bank_auswertung/issues/27); die Entscheidungen A–F waren
> **grillbar** und sind es geworden — [#28](https://github.com/mbalzert1978/bank_auswertung/issues/28)
> (der Split → **A/C revidiert**), [#29](https://github.com/mbalzert1978/bank_auswertung/issues/29)
> (Pager-Motor → **B präzisiert**), [#30](https://github.com/mbalzert1978/bank_auswertung/issues/30)
> (zwei Reichweiten → **E revidiert**), [#31](https://github.com/mbalzert1978/bank_auswertung/issues/31)
> (leere DB → **Konsequenz gepinnt**), [#32](https://github.com/mbalzert1978/bank_auswertung/issues/32)
> (Domänensprache → **Slice `sammeln` in `fenstern` umbenannt**, Begriffe gegen CONTEXT.md geschärft).
> Der Entwurf sprach durchweg von `sammeln`; dieser Text ist auf den beschlossenen Namen `fenstern`
> gezogen.
>
> **Hinweis:** `shared_kernel` ist seit [Issue 0025](../issues/0025-vermeidbare-clones-auf-fehlerpfaden-entfernen.md)
> in `io` aufgelöst; die slice-seitigen Verträge (`Command`/`Handler`/`Publisher`) liegen in
> `application_kernel`.

## Problem

`auswerten` ist im Kern **N→1**: Wiederkehr-Erkennung, Pareto-Ranking und Cashflow sind
**Reduktionen über eine Menge** von Buchungen — eine einzelne `Obfuscated`-Nachricht kann man nicht
sinnvoll auswerten. Die Pipeline stromaufwärts ist dagegen pro-Buchung (`obfuskieren` ist 1:1). Es
fehlt (a) eine **Barriere** „alle relevanten Buchungen liegen vor → jetzt rechnen" und (b) eine Art,
die Analyse über eine potenziell **große Historie** zu **begrenzen und zu paginieren**.

Zweitens hat die Analyse **zwei Reichweiten**, die sich widersprechen, wenn man sie in denselben
Fenster-Schnitt presst: **periodische** Auswertungen (Cashflow/Pareto je Zeitraum) wollen einen
begrenzten Zeit-Slice; **Wiederkehr** ist per Definition **fenster-übergreifend** (ein jährliches Abo
erscheint je ~400-Tage-Seite nur **1×** — isolierte Seiten machen es unsichtbar).

## Entscheidung A — neuer Pipeline-Schritt `fenstern` (Windowing), der ein Aggregat materialisiert

```text
einlesen → obfuskieren → fenstern → auswerten → report
 1→N          1:1          N→1        Fenster + Historie
(Trigger)   (pro Buchung)  (Pager)   (pro Fenster + fenster-übergreifend)
```

`fenstern` ist eine eigene Slice-Triade (`fenstern_domain`/`_adapter`/`_specs`) und ein
Outbox-Consumer wie die anderen — terminal, uniformes ID-Handoff. Der bisher gedachte „auswerten-Slice"
**spaltet** in zwei Triaden: `fenstern` (Fenster bilden) + `auswerten` (Analyse).

**Revidiert ggü. dem Entwurf (#28):** `fenstern` reicht **nicht** bloß einen `{from,to}`-Datumsbereich
weiter — es macht eine echte **N→1-Reduktion** und **materialisiert ein Domänen-Aggregat**:

1. **liest** die N obfuskierten Zeilen des Fensters per Port (`WHERE booking_day BETWEEN from AND to`),
2. **baut** `Auswertungsfenster::new(vec![obfuskierte Zeilen])` — ein echtes Aggregat mit an der
   Konstruktion erzwungenen Invarianten: **eine Währung** ([ADR-0005](0005-domaenenmodell-value-objects-und-summentypen.md))
   und **nicht-leer** (siehe Entscheidung B — jedes Fenster sitzt auf echten Daten),
3. **legt** das Aggregat in **eigener Tabelle mit FKs** auf die obfuskierten Zeilen ab (DB vergibt die ID),
4. **publiziert** die **Aggregat-ID** als Zeiger nach `auswerten`.

Das ist exakt die **uniforme Handler-Form** aller Slices — `read via Port → Domänen-Aggregat bauen →
save (neue Tabelle, neue ID) → ID publizieren` (ID-Handoff, ADR-0012). Die *Kardinalitäten* bleiben
verschieden (`einlesen` 1→N, `obfuskieren` 1:1, `fenstern` N→1), die *Form* ist identisch.

**Was die Materialisierung kauft** (bewusst gegen die Datenduplikation): **Snapshot-Stabilität** (das
Aggregat friert ein, welche Zeilen zum Fenster gehören — ein späterer Import verschiebt die Analyse
nicht rückwirkend), **Barriere als Daten** („dieses Set ist vollständig" wird ein persistiertes Faktum)
und **Determinismus bei Redelivery** (die Pipeline ist at-least-once + wiederaufsetzbar; ein
eingefrorenes Aggregat liest bei erneuter Zustellung deterministisch dieselbe Menge, eine
`BETWEEN`-Range könnte driften). **Vorbehalt:** der FK-Snapshot ist nur ein echter Snapshot, **weil**
die obfuskiert-Zeilen append-only/unveränderlich sind (uniformes ID-Handoff) — wird `obfuskiert` je
mutierbar, bricht die Garantie still.

## Entscheidung B — `fenstern` ist ein trigger-getriebener, sich selbst fortsetzender Pager

- **Zwei Kommandos, zwei Handler** (statt eines Handlers mit `Option<anchor>`), je ein
  `application_kernel::Handler`:
  - **Anstoß (feldlos) — `StartAnalysis`:** ein reiner Auslöser auf der Outbox, **derselbe
    Kickoff-Mechanismus wie `einlesen`** (feldloser Trigger, Decoder immer `Some`). Dem Slice ist egal,
    **wer** ihn einreiht — **heute manuell** (`main`/CLI „auswerten"), **später** io's Broker beim
    Leerlaufen des Imports. Der **Start-Handler** liest **einmalig** `max(booking_day)` aus der DB
    (Port `WindowAnchor`, Entscheidung D) und publiziert `AdvanceWindow{anchor: max}`.
  - **Fenster-Schritt — `AdvanceWindow{anchor}`:** trägt **immer** einen Anker (nie optional). Der
    **Window-Handler** liest die N Zeilen der Seite `[anchor − window_days + 1 … anchor]`, baut und legt
    das `Auswertungsfenster`-Aggregat ab (DB vergibt `window_id`) und macht in **derselben** Transaktion
    **zwei Publishes** über **zwei Sink-Ports**:
    1. **downstream → `auswerten`:** `FensterGeschlossen{window_id}` — der Zeiger auf das abgelegte
       Aggregat (Tabelle = Wahrheit).
    2. **self-continue oder Abschluss — der nächste Anker ist `max(booking_day WHERE booking_day <
       from)`:**
       - liefert die Query **einen** Tag → `AdvanceWindow{anchor: dieser Tag}` (die Kette geht weiter);
       - liefert sie **nichts** → **`HistorieVollständig{Lauf-Spanne}`** (die Historie ist erschöpft,
         siehe Entscheidung E) statt eines Self-Continue.

    Store + **beide** Publishes committen **atomar** zusammen oder gar nicht. Die Unit-of-Work trägt das
    bereits: `einlesen` publiziert schon **N** Nachrichten je Lauf in **einer** Unit; neu ist nur, dass
    die zwei Publishes **verschiedene** `message_type`s haben (der generische `publish<C: Command>` nimmt
    heterogene Typen).
- **Fenster = Seitengröße des Pagers** (nicht der Berichtszeitraum): `window_days` kommt aus der
  **AppConfig** (real **~400** — bewusst **> 365**, damit ein voller Jahreszyklus in **eine** Seite
  fällt); **io liest die Config** und reicht sie dem Handler als Wert herein — genau wie den `seed` bei
  `obfuskierung` (die Domäne liest keine Datei). Cashflow wird **innerhalb** der Seite pro Monat
  gebucketet.
- **Warum die Zwei-Kommando-Form (ADR-0005):** kein `Option<anchor>`, dessen `None` „lies aus der DB"
  bedeutet — der **Typ** trägt die Absicht (feldloser `StartAnalysis` startet, `AdvanceWindow` trägt
  garantiert einen Anker). Der illegale Zustand „leerer Anker" ist **unkonstruierbar**, nicht „geprüft".
  Jeder Handler hat **eine** Aufgabe; zwei `message_type`s, zwei Consumer im selben Slice, vom Dispatcher
  uniform aufgelöst. *(Beim Grilling erwogen und verworfen: den Anker im initialen Kommando mittragen, um
  den zweiten Handler zu sparen — dann müsste irgendwer `max(booking_day)` **vor** dem Kommando
  berechnen, was den feldlosen Trigger/Broker aufweicht.)*
- **Anker im Transport:** `AdvanceWindow` trägt seinen Tag-Stamp als stringifizierten `partition_key`,
  **kein serde** — wie `obfuskieren` die id trägt.

**Präzisierung 1 (#29) — Idempotenz allein über die UnitOfWork.** Unter at-least-once reißt die
Selbst-Fortsetzung **kein neues** Idempotenz-Loch auf, **weil** der Consume-Ack **atomar mit**
store + Publishes committet: eine echte Wiederzustellung passiert nur, wenn die vorige Transaktion
**nicht** committet hat — dann wurde nichts abgelegt/publiziert, also keine gegabelte Kette. Die
UnitOfWork ist dabei **kein fester Tupel** („Ack + store + genau 2 Publishes"), sondern ein
**Decorator über beliebige darunterliegende DB-Bewegungen**, der sie in **einer** Transaktion ausführt
oder gar nicht. Ein zusätzlicher Anker-Unique-Constraint ist **bewusst gestrichen** (Over-Engineering —
mit korrekter UoW feuert er nie und brächte nur einen dritten „schon-erledigt"-Ausgang in den Handler).

**Präzisierung 2 (#29) — Terminierung/Anchoring über `max(booking_day < from)`.** **Ein** Mechanismus
statt `from−1` + `has_bookings_before`: der nächste Anker ist der **jüngste echte Buchungstag unter der
Fensterunterkante**.

- **Dieselbe Query-Art** wie der Initial-Anker (`max(booking_day)`), nur mit Untergrenze — Kickoff,
  Fortsetzung und Terminierung aus **einer** Query-Art.
- **Jedes Fenster sitzt auf echten Daten → nie leer.** Lücken (auch > `window_days`) werden
  **übersprungen** statt in leere Aggregate gegossen; keine Überlappung; jede Buchung landet in genau
  einem Fenster. (Das trägt die **nicht-leer**-Invariant von `Auswertungsfenster`, Entscheidung A.)
- **Terminierung natürlich:** `max(< from)` liefert nichts → Kette endet mit `HistorieVollständig`.
  Kein Zähler, kein Boolean, kein Watermark-Sonderfall.
- **Behebt ein Loch der Entwurfs-Form:** `has_bookings_before(from) = true` garantierte **nicht**, dass
  das nächste `[from−400 … from−1]` nicht leer ist (Datenlücke > `window_days` → leeres Aggregat).

**Präzisierung 3 (Bau 0037) — der Lauf-Gipfel reist im Kommando mit.** `AdvanceWindow` trägt
**zwei** Tag-Stamps im `partition_key` (`"{anchor}:{run_top}"`): den Anker der Seite und den
**Lauf-Gipfel** — den Anker, mit dem der Lauf begann. Grund: der Handler ist **zustandslos** und
kennt sonst nur seine eigene Seite, bräuchte für die **Lauf-Spanne** des abschließenden
`HistorieVollständig` aber deren **Obergrenze**. Die Alternativen waren schlechter — die Spanne der
letzten Seite ist **nicht** die Lauf-Spanne (sie scopte `auswerten`s fenster-übergreifende Query
falsch), und `max(booking_day)` am Ende erneut zu lesen wäre eine zweite Query auf eine
**lebendige** Sicht (ein Nach-Import dazwischen verschöbe die Spanne). Der Gipfel reist unverändert
durch die ganze Kette; der Kickoff (Entscheidung B, `StartAnalysis`) setzt ihn gleich dem
Initial-Anker. **Kein serde** — dieselbe Zwei-Stamp-Form nutzt `HistorieVollständig` schon für die
Spanne.

**Dimensionierung (real):** Input ~3 Jahre (≈ 1095 Tage), Seite ~400 Tage → **~3 Seiten**. Der Pager
feuert wenige Male, dann erschöpft — die Kosten liegen nicht in der Seitenzahl.

> **Verworfen: per-Insert-Trigger** (`fenstern` konsumiert jede `Obfuscated`, prüft „genug Rows → go").
> Zwei Gründe: (1) N Queries; (2) bei einem **Zeit**fenster ist die Abschlussbedingung ein Watermark
> „eine Buchung datiert **nach** dem Fensterende traf ein" — ein **endlicher** Bank-Export liefert das
> für das letzte Fenster **nie**, es bräuchte trotzdem einen expliziten Flush. Per-Insert kostet mehr
> **und** löst das Problem nicht.

## Entscheidung C — Buchungsdatum als sortierbarer Tag-Stamp in der DB

Jedes Datumsfenster braucht auf der obfuskiert-/Analyse-Seite ein **sortier-/abfragbares** Datum. Heute
dekodiert `einlesen` das Datum strukturiert (`Line.booking_day: CalendarDay`), an der obfuskieren-Grenze
wird es aber wieder zu einem **opaken String** plattgedrückt (`IngestedRow.booking_day: String`).
Festlegung: das Datum wird als **tag-genauer Integer-Stamp** persistiert (Mitternacht UTC bzw. „Tage
seit Epoche") — numerisch sortierbar, `MAX()`/Range-Query trivial, Fenster-Arithmetik `± days`. Feste
Zone (UTC) hält es deterministisch. Der Stamp ist die Query-Achse; **`fenstern`** braucht ihn für die
Range-Query, mit der es das Aggregat baut (und für `max(booking_day)` / `max(< from)`); `auswerten`
rekonstruiert für die Domänensicht (Monats-Bucketing) wieder `CalendarDay`. Die **Persistenzform** ist
ein **io-Arbeitspaket** (siehe „Bewusst vertagt").

## Entscheidung D — Anker hinter einem Port (`WindowAnchor`), Implementierung `max`

Der Handler nimmt `anchor: CalendarDay` aus dem Port `WindowAnchor` und weiß **nicht**, ob das `today`
oder `max` ist; die TestApi fakt ihn mit festem Datum. **Gewählt: `max(booking_day)`** — `SELECT
MAX(<stamp>)`, eine indizierte Query, **datenrelativ**. Richtig für **historische Exporte** (ein
`today − 30d`-Fenster liefert bei alten Daten nichts) und **deterministisch** (gleiche DB → gleiches
Fenster, testbar ohne Fake-Uhr). Die Alternative `today` (ein `Clock`-Port) bleibt nachrüstbar, weil
beide hinter demselben Port sitzen — dieselbe Query-Familie trägt dann auch die Terminierung
(`max(< from)`, Entscheidung B).

## Entscheidung E — `auswerten` bedient zwei Reichweiten über **dieselben eingefrorenen Aggregate**

**Revidiert ggü. dem Entwurf (#30): kein Fold-Log / kein `Vorkommens-Log`.** Der Entwurf ließ jeden
Fenster-Lauf `(Fingerabdruck → (Tag, Betrag))` an ein akkumulierendes Log anhängen. Das ist gestrichen —
es wäre eine **zweite persistierte „gesammelte" Sicht** neben dem Aggregat (Redundanz-Wachhund aus #28)
und risse unter at-least-once ein Idempotenz-/Dedupe-Problem auf. Stattdessen: `auswerten` bedient beide
Reichweiten als **Reads über dieselben eingefrorenen `Auswertungsfenster`-Aggregate** — dadurch sind
beide Reichweiten **untereinander konsistent** und **replay-deterministisch**, und es gibt **gar keine**
zweite gespeicherte Sicht.

`auswerten` hat **zwei einzweckige Handler** (symmetrisch zu `fenstern`s zwei Handlern):

1. **Pro-Fenster — Handler auf `FensterGeschlossen{window_id}`:** liest **das eine** Aggregat per ID und
   rechnet die periodische Analyse **innerhalb** des Zeitraums — Cashflow je Zeitraum,
   Gebühren-Klassifikation, Pareto **innerhalb** des Fensters.
2. **Fenster-übergreifend — Handler auf `HistorieVollständig`:** eine **Read-Port-Query über alle
   `Auswertungsfenster`-Aggregate des Laufs** (die mitgetragene Lauf-Spanne scopt sie auf genau diesen
   Lauf) — **synchron, terminal, im Handler**. Die **Wiederkehr-Erkennung** liest daraus die
   `(Fingerabdruck, Richtung, Betrag, Tag)`-Vorkommen; das **Netting** (Umbuchungen raus,
   [Issue 0005](../issues/0005-wiederkehr-erkennung.md) „arbeitet auf genetteten Abflüssen") läuft **in**
   der Query-Aktion vor der Rhythmus-Prüfung. Die Query darf optional auf **Kostentreiber-Fingerabdrücke**
   (Pareto-Top-N) gefiltert werden — Wiederkehr zählt vor allem für Kostentreiber.

**Der Abschluss-Trigger `HistorieVollständig` ist nötig, weil** die Wiederkehr-Query **einmal, bei
vollständiger Historie** laufen muss, `auswerten` die letzte Seite aber **nicht selbst** erkennen kann:
der Pager läuft **neu→alt**, und die Erschöpfungs-Kenntnis lebt nur in `fenstern` (wo `max(< from)`
nichts liefert). Darum publiziert `fenstern` auf dem Terminierungs-Zweig `HistorieVollständig` (atomar
mit der letzten Fenster-Ablage) — bei Ankunft sind **alle** Aggregate des Laufs da.

**Warum die fenster-übergreifende Sicht überhaupt nötig ist:** ein `Auswertungsfenster` validiert/sichert
**eine** ~400-Tage-Seite; darin steckt **monatliche** Wiederkehr vollständig, ein **jährliches** Abo aber
nur **1×** (an den Rändern evtl. 2×, unzuverlässig). „Jährlich wiederkehrend"
([Issue 0005](../issues/0005-wiederkehr-erkennung.md), Jahreskosten) braucht **2+ Vorkommen im
12-Monats-Abstand** über die Seiten hinweg → nur die Sicht **über alle** Aggregate belegt es. Kein
einzelnes Aggregat kann das beantworten — es ist eine **Zwischen-Seiten**-Frage.

> **Verworfen (#30), mit Grund:**
>
> - **Gespeichertes Fold-Log** — zweite persistierte Sicht (Redundanz-Wachhund) + Dedupe/Idempotenz-
>   Maschinerie unter at-least-once. Der Read über unveränderliche Aggregate löst beides ersatzlos.
> - **Direkt gegen die `obfuskiert`-Tabelle queren** — gäbe der Wiederkehr eine **lebendige** Sicht, die
>   bei Nach-Import/Replay vom eingefrorenen Pro-Fenster-Reach **abweicht** (die Konsistenz, für die #28
>   die Aggregate materialisiert hat), und überspränge die Pipeline-Kante (`auswerten`s Vorgänger ist
>   `fenstern`, nicht `obfuskierung`).
> - **Bus-`ExpandWindow`-ASK** (`auswerten` bittet `fenstern`, ein Aggregat um einen Kandidaten zu
>   „strecken" und zurückzuliefern) — bräuchte eine **Rückwärtskante**, **synchrones Request/Reply**
>   (gegen ADR-0008 „kein Next-Call") und **persistenten Fortsetzungs-State** (gegen ADR-0012 „durchweg
>   terminal"). Datenholen ist ein **Port** (synchron, terminal), der Bus koppelt nur Schritte.

## Entscheidung F — was `auswerten` sonst besitzt (aus 0004–0011 übernommen)

- **Fingerabdruck** als reine Domänenfunktion (inkl. Zahlungsintermediär-Erkennung + Händler-Token),
  [ADR-0004](0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)/[0006](0006-pseudonym-stabilitaet.md).
- **`Gegenkonto::Eigen|Fremd` auf Token** — Gegenkonto-`KONTO_x` gegen die mit **demselben Seed**
  tokenisierte Menge der eigenen Konten; braucht die **Eigene-Konten-Config**
  ([Issue 0002](../issues/0002-eigene-konten-config-schema-und-loader.md), token-orientiert, IBAN + Rolle).
- **`Kategorisierer`** nur für der Domäne **unbekannte** Fingerabdrücke (LLM-Orakel, Export→Chat→Import);
  Egress-DTO `KategorieAnfrage` trägt die `assert_not_impl_any!`-„kein `Serialize`"-Sperre.

## Konsequenzen / Vorbedingungen

- **Leeres Fenster / leere DB (#31): `Ok(())`, kein Fehler.** Ist die DB leer, liest der Start-Handler
  `max(booking_day) = None` → **`Ok(())`**, **publiziert nichts** und triggert `auswerten` **nicht**
  (Erschöpfung bereits bei Schritt 0, symmetrisch zum normalen Ketten-Ende `max(< from)` = leer). **Kein**
  Dead-Letter (den gibt es nicht, ADR-0012), **kein** „nichts auszuwerten"-Fehler. Verstärkt durch #30:
  weil kein Kopf-`HistorieVollständig` mit leerer Lauf-Spanne entsteht, bleibt dessen Spanne **garantiert
  nicht-entartet** (make-illegal-states-unrepresentable). Trade-off: ein Leer-Lauf ist **still** —
  „lief / lief-nie" unterscheidet die (vertagte) Report-Schicht, nicht die Pipeline.
- **io-Vorbedingungen:** (1) sortierbarer Tag-Stamp in der obfuskiert-Ablage (Entscheidung C); (2) wer
  `StartAnalysis` einreiht (heute manuell; später Broker-Drain beim Leerlaufen des Imports).
- **Die Fach-Issues hängen als Analyzer-Bausteine unter `auswerten`:**
  [0004](../issues/0004-umbuchung-netting.md) (Umbuchung-Netting),
  [0005](../issues/0005-wiederkehr-erkennung.md) (Wiederkehr, liest die fenster-übergreifende Query),
  [0006](../issues/0006-gebuehren-klassifikation.md) (Gebühren, pro Fenster) und
  [0007](../issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md) (Pareto/Sparkandidaten, pro
  Fenster + speist optional den Wiederkehr-Filter). [Issue 0002](../issues/0002-eigene-konten-config-schema-und-loader.md)
  (Eigene-Konten-Config) wird ein **`auswerten`-Input** (token-orientiert).

## Bewusst vertagt

Das echte `io` (Broker-Auto-Trigger beim Drain, `rusqlite`-Stores (synchron,
[ADR-0014](0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)), die reale Tag-Stamp-Persistenz aus
Entscheidung C), der `report` (u. a. die „lief / lief-nie"-Unterscheidung), persistente Seed-Verwaltung
und die `ObfuskierterText`-Scrubbing-Regeln.

## Domänensprache (geschärft, #32)

Die neuen Begriffe stehen in [CONTEXT.md](../../CONTEXT.md). **Glossar (Domänen-Konzepte):**
`Auswertungsfenster` → Code `AnalysisWindow`; `fenstern` → Code `Windowing` (der Slice; `sammeln`/
`Collector` ist verworfen, jetzt `_Avoid_`, weil der Schritt nichts von außen sammelt — er **bildet** das
Fenster). **Zwei neue Markierte Mehrdeutigkeiten:** *Fenster (Pager-Seitengröße ~400 d) ≠ Zeitraum
(Monats-Bucketing/Bericht)* und *Reichweite pro-Fenster ≠ fenster-übergreifend*. Die **Pipeline-Mechanik**
`StartAnalysis`, `AdvanceWindow`, `FensterGeschlossen`, `HistorieVollständig`, `WindowAnchor` lebt
bewusst **nur hier im ADR** (englische Code-Namen, kein Glossar-Eintrag); `Vorkommens-Log` ist verworfen.
