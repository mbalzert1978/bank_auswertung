# Dead-Letter als Status-Spalte der Outbox

> **Status: Akzeptiert, gebaut** (Issue
> [0085](../issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)).
> Form vorgegeben im [Abschlussplan](../plans/2026-10-12-1343-abschlussplan.md), Schritt 4c; die
> Lauf-Folgen hat der Orchestrator in Review-Runde 1 entschieden (2026-10-12).

Dieses ADR beantwortet die Frage, die
[ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) ausdrücklich vertagt hat:
**wie `io` einen terminalen Fehlschlag surfacet.** Es verfeinert außerdem
[ADR-0016](0016-zeilen-schluessel-und-lauf-eigene-fenster.md) an einer Stelle (siehe „Folgen für
ADR-0016"). Die Pipeline-Garantien von
[ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) (atomar, at-least-once, wiederaufsetzbar),
die Datenschutz-Grenze an `obfuskieren` und der Slice-Vertrag von
[ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) bleiben unberührt: **kein Slice lernt
Retry oder Dead-Letter kennen.**

## Problem

Eine Nachricht, deren Handler terminal scheiterte, blieb in der SQLite-Outbox **offen**. Jeder
spätere Lauf stellte sie erneut zu und scheiterte erneut; ein Lauf mit einer anderen Datei brach an
ihr mit `RunError::PendingImport` ab. Dasselbe galt für einen dauerhaft scheiternden Commit. Niemand
zählte Versuche. Eine einzige vergiftete Nachricht hielt so jeden späteren Lauf auf.

## Entscheidung A — eine Status-Spalte in der Outbox, keine eigene Tabelle

- Die Outbox-Zeile trägt `status` ∈ `open`/`processed`/`dead_letter` und `dead_letter_reason`.
  Zugestellt wird nur `open`. Eine ausgesonderte Nachricht **bleibt dieselbe Zeile**; sie wandert
  nirgendwohin.
- Zwei Tabellen-`CHECK`s machen die illegalen Kombinationen unkonstruierbar
  ([ADR-0005](0005-domaenenmodell-value-objects-und-summentypen.md)): `processed` genau dann, wenn
  ein Ack-Stempel steht; `dead_letter` genau dann, wenn ein Grund steht.
- `attempt` zählt **jeden** Zustellversuch, auch den gelungenen.
- Warum keine eigene Tabelle: eine zweite Tabelle hieße, die Nachricht zwischen zwei Orten zu
  verschieben — eine Transaktion mehr, und zwei Stellen, die „was ist mit Nachricht X" beantworten.
  Der Status an der Zeile ist der übliche Weg und reicht.
- Die Schema-Änderung ist die Sprosse **3 → 4** der Migrations-Leiter (`SCHEMA_VERSION = 4`), mit
  eingefrorener Ziel-DDL; der Bestand bleibt (`processed_on_utc` gesetzt → `processed`, sonst
  `open`; die `rowid` und damit die Zustell-Reihenfolge reist mit).

## Entscheidung B — terminal sofort, transient nach einer Höchstzahl

- **Der Broker entscheidet**, nicht der Consumer und nicht der Slice. Die Regel ist eine reine
  Funktion (`disposition`) über den `OutboxError` des Versuchs und die bisherigen Versuche:
  - `Warning` (der Slice scheiterte) und `Critical` (die Hülle trug nicht) → **sofort**
    ausgesondert. Wiederholen änderte nichts.
  - `Retry` (der Commit scheiterte) → offen lassen, bis `MAX_DELIVERY_ATTEMPTS` (3) Versuche
    verbraucht sind, dann aussondern.
- **Eine Konstante, keine Config-Datei** (YAGNI): der Wert ist Broker-Mechanik, kein Fachparameter.
- Der **Grund** ist die `Display` des `OutboxError`, also der nackte, redigierte Domänen-Fehler des
  Slice, eine Transaktions-Diagnose oder die opake Message-ID samt Dispatch-Key — nie `partition_key`
  oder `content`.
- Was der Broker meldet, kommt **aus der getroffenen Zeile** (`UPDATE … WHERE status = 'open'
  RETURNING …`). Trifft das `UPDATE` keine offene Zeile mehr (eine andere Zustellung hat sie schon
  erledigt), meldet er genau das, statt einen Versuch zu erfinden. Scheitert das Verbuchen selbst,
  gehen die schon verbuchten Ausgänge des Aufrufs nicht verloren; der Broker hört dann auf
  zuzustellen.

## Entscheidung C — was der Lauf daraus macht

- **Beide Phasen laufen zu Ende.** Die gesunden Nachrichten hinter einer vergifteten werden weiter
  zugestellt, und die Analyse findet statt.
- **Ein Lauf, der selbst etwas aussondert, endet mit Exit 1** (`RunError::DeadLettered`) und nennt
  jede solche Nachricht redigiert. Nichts wird still verschluckt.
- **Ein späterer Lauf wird nicht aufgehalten.** Eine ausgesonderte Nachricht ist nicht offen: sie wird
  nicht erneut zugestellt und löst kein `PendingImport` aus. Das greift nur noch bei einem wirklich
  offenen Auslöser, also nach einem Absturz. Die Schlussbilanz zählt ausgesonderte Nachrichten
  früherer Läufe als „früher ausgesondert" weiter mit.
- **Eine fehlende Buchung bleibt sichtbar — der Zustand kommt aus der Datei, nicht aus dem Lauf.**
  Ein ausgesondertes `Ingested` fehlt **für immer** in `obfuskiert`: eine bekannte Zeile wird nie
  neu publiziert (ADR-0016). Jeder Lauf zählt darum am Ende die ausgesonderten `Ingested` der
  **Datei** und gibt sie öffentlich als `RunReport::incomplete` heraus; das Protokoll schreibt
  „Analyse unvollständig: n Buchung(en) ausgesondert". Der Lauf endet trotzdem mit Exit 0, wenn er
  selbst nichts ausgesondert hat — ein Dauer-Exit-1 hieße, dass es nie wieder einen Bericht gibt. Der
  Bericht ([Issue 0012](../issues/0012-html-bericht.md)) zeigt die Zahl an.
- Ein ausgesonderter **Import-Auslöser** (`ReadTransactions`) zählt **nicht** als unvollständig: sein
  Batch ist atomar, es fehlt dann keine einzelne Buchung, sondern der ganze Auszug. Der Lauf, der ihn
  aussonderte, endet mit Exit 1 und nennt ihn; ein erneuter Import der korrigierten Datei füllt die
  Lücke. Ihn dauerhaft zu zählen, meldete nach diesem Import eine Lücke, die es nicht mehr gibt.

## Folgen für ADR-0016

ADR-0016 sagte, ein Lauf, der mitten in `fenstern` oder `auswerten` scheitert, bleibe unvollständig,
„bis der nächste Programmstart ihn zu Ende führt". Das stimmt nur noch **nach einem Absturz**. Ein
ausgesondertes `AdvanceWindow` oder `WindowClosed` lässt seinen Lauf **für immer** unvollständig.
Die Grenze aus ADR-0016 trägt das schon: der Bericht liest nur die Lauf-Identität, die der eigene
Exit-0-Lauf gerade gemintet hat, und ein Lauf mit Aussonderung endet mit Exit 1.

## Bewusst nicht gebaut

- **Kein Redrive-Werkzeug** (YAGNI). Eine ausgesonderte Nachricht zurück in die Zustellung zu holen,
  kann das Programm nicht. Von Hand geht es nur über SQL, und zwar mit Status **und** Grund
  (`status = 'open', dead_letter_reason = NULL`), weil der `CHECK` beides verlangt. Ob es je gebraucht
  wird, ist offen: der übliche Weg ist, die Ursache zu beheben und den Auszug neu einzulesen.
- **Kein Aufräumen.** Ausgesonderte Zeilen bleiben für immer liegen, wie verarbeitete.
- **Kein Lauf-Bezug an der Outbox-Zeile.** Welche Aussonderung zu welchem Lauf gehört, weiß nur
  dieser Lauf selbst (er sammelt seine). Für „früher ausgesondert" reicht die Differenz.
