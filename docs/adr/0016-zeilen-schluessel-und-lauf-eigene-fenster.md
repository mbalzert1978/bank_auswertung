# Zeilen-Schlüssel, lauf-eigene Fenster und vorgemerkte Umsätze überspringen

> **Status: Akzeptiert, gebaut** (Issue
> [0088](../issues/0088-zeilen-schluessel-lauf-eigene-fenster-und-vorgemerktes-ueberspringen.md)).
> Rahmen mit Markus abgestimmt im
> [Abschlussplan](../plans/2026-10-12-1343-abschlussplan.md), Schritt 2.

Dieses ADR macht **mehrere Läufe in eine Datenbank** richtig: je eigenem Konto ein Lauf, auch
wiederholt. Es verfeinert [ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
(uniformes ID-Handoff — die Persistenz vergibt weiter jede ID, dazu kommt eine **fachliche**
Identität der Zeile) und [ADR-0013](0013-zeitfenster-collector-paginierender-analyse-kopf.md)
(Pager `fenstern` und die fenster-übergreifende Reichweite — der Scope ist jetzt der **Lauf**, nicht
die Spanne). Die Pipeline-Garantien (atomar, at-least-once, terminal) und die Datenschutz-Grenze an
`obfuskieren` bleiben unberührt. Es ist die **erste echte Migration** auf dem Versionsanker aus
Issue [0084](../issues/0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md).

## Problem

Vor diesem ADR ergaben mehrere Läufe in **eine** Datei falsche Zahlen, aus drei Gründen:

1. **Kein Zeilen-Schlüssel.** Jede Ergebnis-Tabelle trug nur ihre generierte `id`. Wer denselben
   Auszug zweimal einlas, hielt den Bestand zweimal.
2. **Fenster ohne Lauf.** `fenstern` paginiert je Lauf die **ganze** obfuskierte Historie neu, und
   die fenster-übergreifende Abfrage von `auswerten` scopte nur über die Lauf-**Spanne**. Die Fenster
   früherer Läufe liegen aber in derselben Spanne — sie zählten erneut mit (gemessen: 53 → 159 → 318
   Fenster-Buchungen über drei Läufe derselben Datei).
3. **Vorgemerkte Zeilen brachen den Import.** Eine Zeile mit `Info` = `Umsatz vorgemerkt` hat ein
   leeres Valutadatum, `0` als IBAN/BIC und keine Kundenreferenz. Die Valuta ist Pflichtfeld — die
   **ganze** CSV scheiterte terminal.

## Entscheidung

### A — Jede eingelesene Zeile trägt einen Zeilen-Schlüssel

- **Zusammensetzung** (Rahmen von Markus):
  - ist die **Kundenreferenz (End-to-End)** gefüllt: **Auftragskonto + Kundenreferenz** (dazu Betrag
    und Buchungstag, siehe unten);
  - sonst **Rückfall**: Auftragskonto + **alle Spalten** der Zeile (darunter der Buchungstag);
  - in beiden Fällen dazu die **laufende Nummer gleicher gebuchter Zeilen innerhalb des Imports**
    (die erste gleiche Zeile trägt 1, die zweite 2). So zählen zwei echte, gleiche Buchungen am
    selben Tag beide, und ein überlappender Auszug, der sie wieder trägt, erzeugt dieselben
    Schlüssel.
  - In Code ein Summentyp (`RowIdentity::Reference { account, reference, amount, day }` |
    `RowIdentity::AllColumns(..)`) mit `digest(occurrence)`.
- **Detail: der Betrag gehört zum Kundenreferenz-Schlüssel.** Eine Rücklastschrift wiederholt die
  Kundenreferenz der ursprünglichen Lastschrift. Ohne den (vorzeichenbehafteten) Betrag erkennte ein
  späterer Auszug die Rückbuchung als „schon bekannt" und verschluckte sie. Der Betrag einer
  gebuchten Zeile ändert sich zwischen zwei Exporten nicht; er kostet also nichts.
- **Detail: der Buchungstag gehört zum Kundenreferenz-Schlüssel** (Review-Runde 1, Etikett
  `kundenreferenz/2`). Manche Gläubiger nutzen Monat für Monat **dieselbe** Referenz zum selben
  Betrag. Ohne den Tag zählte die laufende Nummer über die ganze Datei: Auszug A (April–Juni) und
  Auszug B (Mai–Juli) erzeugten je die Nummern 1–3, und B's Juli sähe aus wie A's April — er fehlte
  still, obwohl kein Tag abgeschnitten war. Mit dem Tag zählt die Nummer, wie beim Rückfall-Weg, nur
  innerhalb eines Tages.
- **Detail: nur Gebuchtes wird durchgezählt.** Der Buchungsstand steht **vor** der Nummerierung fest;
  eine vorgemerkte Zeile bekommt **keinen** Schlüssel und verschiebt keine Nummer. Sonst trüge
  dieselbe gebuchte Zeile neben ihrem vorgemerkten Zwilling eine andere Nummer als im nächsten
  Auszug, in dem der Zwilling gebucht ist.
- **Detail: `NOTPROVIDED` zählt als leer.** Das ist der SEPA-Platzhalter für „keine Referenz";
  sonst teilten sich alle Zahlungen ohne Referenz eines Kontos **einen** Schlüssel.
- **Die Grenze — für beide Wege dieselbe:** die laufende Nummer zählt gleiche Zeilen eines **Tages**
  innerhalb **einer** Datei. **Ein Tag muss vollständig im Export stehen.** Schneidet Auszug A einen
  Tag ab und trägt Auszug B die übrigen, gleichen Zeilen jenes Tages, beginnen sie in B wieder bei
  Nummer 1 und sehen aus wie die schon bekannten — sie gingen verloren. Überlappen sich zwei Auszüge,
  muss jeder den gemeinsamen Tag **vollständig** tragen. (Steht auch im Hilfetext des Programms.)
- **Wo er entsteht: in `io`, in der CSV-Quelle.** Welche Spalte die Kundenreferenz ist und was „alle
  Spalten" heißt, ist **Format-Wissen** — es gehört hinter den Quell-Seam
  (`.rules/common/architektur.md`). Die Quelle reicht den Schlüssel als opake Zeichenkette im
  `RawRecord` herein; der Slice trägt ihn als `RowKey` durch und **vergleicht** ihn nur. Ein späteres
  Format (MT940, XML-camt) liefert seinen eigenen Schlüssel nach seiner eigenen Regel.
- **Abgelegt wird ein Digest, nie der Klartext.** Der Schlüssel enthält eine IBAN und die
  Kundenreferenz. `einlesen_lines` darf Rohdaten lokal halten
  ([ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)), aber nicht mehr als nötig: die
  Kundenreferenz und die zusammengesetzte Zeile braucht dort niemand. Darum legt die Quelle nur
  **SipHash-1-3, 128 Bit** über die längen-präfixierten Bestandteile ab (32 Hex-Zeichen, dieselbe
  battle-tested Lib wie der Obfuskator). Bewusst mit **festem** Schlüssel, **nicht** mit dem Seed:
  hinge der Zeilen-Schlüssel am Seed, erkennte nach einem Seed-Wechsel kein Import mehr eine schon
  eingelesene Zeile — der Bestand verdoppelte sich still. Der Digest soll nichts verbergen (die Zeile
  liegt daneben ohnehin roh), sondern nur nicht mehr ablegen als nötig. 128 Bit halten eine
  zufällige Kollision, die eine echte Buchung als „bekannt" verwürfe, praktisch aus. Ein Etikett mit
  Versionsziffer (`kundenreferenz/2`, `alle-spalten/1`) trennt die beiden Arten und macht eine
  spätere Änderung der Ableitung sichtbar.
- **`einlesen_lines.row_key` ist `NOT NULL UNIQUE`.** Der Index trägt zugleich die Frage vor jeder
  Ablage.
- **Doppelte Schlüssel in einem Batch sind ein Vertragsbruch der Quelle**: die ACL des Slice weist
  sie terminal und redigiert ab (`SourceError::DuplicateRowKey`, datenlos), statt eine echte Buchung
  still als „bekannt" zu verwerfen. Ein fehlender Schlüssel ist ein fehlendes Pflichtfeld
  (`row_key`).

### B — Ein neuer Import ergänzt

- Der `Store`-Port antwortet mit einem Summentyp: `Recording::New(IngestionId)` oder
  `Recording::AlreadyKnown`. Der Handler publiziert **nur** bei `New` — eine schon bekannte Zeile wird
  weder erneut angelegt noch erneut publiziert, sonst legte `obfuskieren` sie zweimal ab.
- Gefragt wird der **committete** Bestand (`SELECT EXISTS … WHERE row_key = ?`), außerhalb des
  Commit-Rands wie das Minten der ID. Gleiche Schlüssel innerhalb eines Batches fängt schon A ab; die
  `UNIQUE`-Spalte bleibt das letzte Netz.
- Ein Import **ersetzt** nichts und löscht nichts (Entscheidung von Markus: „ergänzen").
- Der Import-Auslöser bleibt, wie er ist: ein liegengebliebener Auslöser einer **anderen** Datei
  stoppt den Lauf weiterhin (`RunError::PendingImport`); derselbe Auszug ein zweites Mal ist jetzt
  harmlos.

### C — Fenster und Auswertungen gehören zu genau einem Lauf

- **Jeder Lauf paginiert weiter die ganze obfuskierte Historie** — aller Konten. Genau deshalb zeigt
  der **jüngste** Lauf alle Konten gemeinsam, und derselbe Lauf mehrfach ergibt dieselben Zahlen.
- **Die Lauf-Identität entsteht dort, wo der Lauf beginnt:** `io::run` mintet sie beim Anstoß der
  Analyse an der DB und reiht sie als `partition_key` des `StartAnalysis` ein. Der Anstoß ist eine
  Wurzel-Nachricht des Laufs, so wie der Import-Auslöser seine Quelle benennt.
- **Sie trägt die Ordnung der Läufe — „der jüngste Lauf" ist ein Vertrag:** die Identität beginnt mit
  dem UTC-Startzeitpunkt auf die Millisekunde in fester Breite (`20261012T134501.123`), gefolgt von
  64 Bit Zufall. **Der jüngste Lauf ist der mit der lexikografisch größten `run_id`.** Bewusst nicht
  SQLites `rowid` (ein `VACUUM` darf sie neu vergeben) und keine eigene Lauf-Nummer-Tabelle. Läufe auf
  einer Datei sind seriell und dauern länger als eine Millisekunde; zwei gleiche Zeitstempel gibt es
  nicht. Die Grenze: eine rückwärts gestellte Systemuhr bräche die Ordnung — das Programm vertraut der
  Uhr der Maschine. Den Vertrag nutzt der Bericht (Issue 0012).
- **„Jüngster Lauf" heißt jüngster abgeschlossener Lauf.** Scheitert ein Lauf mitten in `fenstern`
  oder `auswerten` terminal, sind seine bisherigen Fenster schon committet — `max(run_id)` zeigte dann
  auf einen **unvollständigen** Lauf, bis der nächste Programmstart ihn zu Ende führt. Darum gilt
  (Entscheidung des Orchestrators, 2026-10-12): der **Bericht entsteht am Ende eines Laufs, der die
  Outbox leer laufen lässt (Exit 0), und liest genau die Lauf-Identität, die dieser Lauf gerade
  gemintet hat** — er sucht den Lauf nicht nachträglich per `max(run_id)`. Nach Exit 1 gibt es keinen
  Bericht. `max(run_id)` bleibt die Ordnung für Werkzeuge, die die Datei später lesen; sie müssen
  dieselbe Grenze kennen.
- **Sie reist als Wert durch die Nachrichten**, nicht als Envelope-ID — der Slice bleibt
  Outbox-blind ([ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md)):
  - `StartAnalysis` → `"{run}"`,
  - `AdvanceWindow` → `"{anchor}:{run_top}:{run}"`,
  - `HistoryComplete` → `"{from}:{to}:{run}"` (der Lauf steht zuletzt, weil er opak ist).
  In `fenstern` ist sie ein Domänen-Wert (`RunId`): `StartAnalysis` trägt sie (statt feldlos zu
  sein), `AdvanceWindow` reicht sie unverändert weiter, das `AnalysisWindow` gehört ihr, und der
  Abschluss nennt sie. Ein Schlüssel ohne Lauf ist keine gültige Envelope.
- **`auswertungsfenster.run_id`** (`NOT NULL`, indiziert). Die Auswertungen eines Fensters gehören
  über ihre `window_id` demselben Lauf an — eine eigene Lauf-Spalte bräuchten sie nicht.
- **Die fenster-übergreifende Abfrage scopt über den Lauf** (`WHERE w.run_id = ? AND` Spanne).
  Die `auswerten`-**Domäne bleibt lauf-blind**: den Lauf liest der Decoder aus dem Abschluss, die
  Lese-Bridge entsteht je Kommando und legt ihn als Scope an den Seam. Scoping ist
  Beschaffungs-Mechanik (so steht es schon am `OccurrenceSeam`); die Domäne verlangt „die Historie
  dieses Laufs", nicht eine Lauf-ID.
- **Verworfen: eine Lauf-Tabelle, deren ID die Persistenz beim Anstoß vergibt.** Sie folgte dem
  ID-Handoff wörtlicher, brauchte aber eine weitere Tabelle, einen Port, einen Seam und einen Store
  für eine Identität, die außer den Fenstern niemand liest. Die Fenster tragen den Lauf; „der
  jüngste Lauf" ist der mit der größten `run_id` (siehe oben).
- **Ein liegengebliebener Lauf** (Analyse terminal gescheitert) wird beim nächsten Programmstart
  unter **seiner** alten Lauf-Identität zu Ende geführt (in Phase 1, die alle offenen Nachrichten
  abarbeitet); der neue Lauf daneben hat seine eigene. Keiner zählt die Fenster des anderen, und die
  Vorkommens-Zahl im Lauf-Bericht nennt nur die des neuen Laufs.

### D — `einlesen` überspringt `Umsatz vorgemerkt` vor der Pflichtfeld-Prüfung

- Der `RawRecord` trägt einen Buchungsstand (`RecordStatus::Booked | Pending`, Default `Booked`).
  Die CSV-Quelle setzt `Pending` genau für `Info` = `Umsatz vorgemerkt` (Format-Wissen, `io`).
- Die ACL (`SourceBridge`) verwirft vorgemerkte Zeilen **vor jeder Prüfung** — vor Ein-Währung,
  Schlüssel-Eindeutigkeit und Präsenz. Sie bricht nichts ab und zählt nicht; gebucht kommt sie mit
  einem späteren Auszug.
- Folge für die Namensliste des Scrubbings (Issue 0010): sie liest `einlesen_lines`, sieht also nur
  Gebuchtes.

### E — Migration 1 → 2

- `SCHEMA_VERSION = 2`. `schema::ensure` bekommt den Arm `Found::Version(1) => migrate_1_to_2`.
- **Ohne Bestand** (`einlesen_lines` und `auswertungsfenster` leer): beide Tabellen werden
  verworfen und aus der heutigen DDL neu angelegt. Danach trägt die Datei **genau** das Schema einer
  frischen (per Test verglichen) — kein `ALTER TABLE` mit Ersatz-Default, das davon abwiche.
- **Mit Bestand: terminal und redigiert abgewiesen** (`MismatchCause::UnkeyedRows`, nur
  Versionsnummern), die Datei bleibt unangetastet. Ihre Zeilen lassen sich **nicht** nachträglich
  schlüsseln: der Schlüssel entsteht aus Spalten, die Version 1 nie abgelegt hat (Kundenreferenz,
  Mandatsreferenz, BIC, Info …). Ein Ersatzschlüssel aus dem Vorhandenen träfe beim nächsten Import
  desselben Auszugs nie — der Bestand verdoppelte sich still, genau der Fehler, den dieses ADR
  behebt. Eine echte Nutzer-Datenbank gibt es noch nicht; die Meldung verlangt eine frische Datei und
  ein erneutes Einlesen.

## Konsequenzen

- **Richtig:** A, dann B, dann A erneut in eine Datei ergibt dieselben Zahlen des jüngsten Laufs wie
  A und B je einmal (Test in `crates/io/src/run/tests.rs`); ein zweiter Lauf zählt keine Fenster eines
  früheren mit; zwei gleiche Buchungen am selben Tag zählen beide; eine vorgemerkte Zeile bricht
  nichts.
- **Kosten:** die Fenster-Tabelle wächst weiter je Lauf um einen vollständigen Satz Fenster (jeder
  Lauf paginiert die ganze Historie). Das ist gewollt — es hält den jüngsten Lauf in sich vollständig
  — und bei einem privaten Bestand klein. Aufräumen alter Läufe ist eine eigene, spätere Frage.
- **Grenze bleibt Bedienregel:** kein Export schneidet einen Tag mittendrin ab.
- **Seam-Änderungen** (Slice↔`io`): `RawRecord` (+`row_key`, +`status`), `StoreSeam` (→
  `StoreOutcome`), `AnalysisWindowRow` (+`run`), `OccurrenceSeam::history_in` (+`run`), die
  Partition-Key-Formen oben. `application_kernel` bleibt unverändert.
