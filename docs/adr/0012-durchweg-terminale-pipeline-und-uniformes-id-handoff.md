# Durchweg-terminale Pipeline und uniformes ID-Handoff

> **Verfeinert durch [ADR-0016](0016-zeilen-schluessel-und-lauf-eigene-fenster.md):** neben der von
> der Persistenz vergebenen ID trägt jede eingelesene Zeile einen fachlichen **Zeilen-Schlüssel**; eine
> schon bekannte Zeile wird weder erneut abgelegt noch publiziert. Das ID-Handoff selbst bleibt.
>
> **Die vertagte Frage ist beantwortet** durch
> [ADR-0017](0017-dead-letter-als-status-spalte-der-outbox.md): `io` surfacet einen terminalen
> Fehlschlag, indem der **Broker** die Nachricht aussondert — Status-Spalte `dead_letter` in der
> Outbox samt redigiertem Grund; ein gescheiterter Commit wird bis zu einer Höchstzahl wiederholt. Der
> Slice-Vertrag bleibt unverändert.

Dieses ADR **verfeinert/supersedet in Teilen** [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)
(§ „Der Handler arbeitet auf Domänen-Typen", Envelope-Konvention) und
[ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) (Entscheidung B — Fehler-Taktik).
Es korrigiert die **„Dead-Letter"/„io klassifiziert Warning/Retry/Critical"-Erzählung**, die
von Beginn an auf einem **Missverständnis** beruhte, und pinnt das über die Slices
`einlesen`/`obfuskierung` gelebte **uniforme ID-Handoff** als Entscheidung fest. Die
**Pipeline-Garantien von ADR-0008** (`einlesen → obfuskieren → auswerten → report`; atomar,
at-least-once, wiederaufsetzbar; Datenschutz-Grenze an `obfuskieren`) und die
**Slice-Mechanik-Verortung von ADR-0011** (Entscheidung A/C/D: der Slice ist eine reine
`Command → Domäne → Response`-Pipeline, die Outbox-Mechanik + der Kernel-Flip leben in `io`)
bleiben **unberührt maßgeblich**.

Herkunft: die Slice-Issues
[0028](../issues/0028-obfuskierung-terminal-fehlbares-read-save-publish-und-id-handoff.md) /
[0029](../issues/0029-einlesen-terminal-fehlbares-store-publish.md) räumen die verstreute,
teils falsche Rationale aus den Docstrings; dieses ADR (Issue
[0030](../issues/0030-adr-0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)) ist
ihr **kanonischer Ort**.

## Problem

Das über `einlesen`/`obfuskierung` etablierte Muster war bisher **nur in Docstrings**
verstreut beschrieben — teils **falsch** und mit ausufernder Rationale, die nicht in
Docstrings gehört. Zwei Dinge brauchen einen kanonischen Ort:

1. **Fehler-Taktik.** ADR-0011, Entscheidung B, rahmte jeden Handler-Fehler als „terminal →
   **Dead-Letter**" und ließ `io` ihn in `Warning`/`Retry`/`Critical` **klassifizieren**
   (inkl. attempt-basierter Eskalation Retry → Dead-Letter). Das „Dead-Letter" und die
   `io`-Klassifikation beruhten auf einem **Missverständnis von Beginn an**: die Pipeline ist
   schlicht **durchweg terminal**. Scheitert ein Schritt, ist das terminal — es gibt **kein**
   Dead-Letter-Konzept und **kein** Retry im Slice. Wie `io` (vertagt) einen terminalen
   Fehlschlag nach außen surfacet, ist **dort** offen und **keine** Slice-Frage.
2. **ID-Handoff.** Das Muster „lies den Vorgänger per ID → baue dein Aggregat → transformiere
   → schreibe in deine **eigene** neue Tabelle → publiziere **diese neue** ID als Zeiger" ist
   das richtige, uniforme Verhalten (`einlesen` lebt es bereits), war aber nirgends als
   Entscheidung festgehalten — sodass Abweichungen (eine unfehlbare Ablage, die die
   **Quell-ID** statt der neu vergebenen ID publiziert) unauffällig einsickern konnten.

## Entscheidung A — die Pipeline ist durchweg terminal

- **Alle drei I/O-Schritte jedes Slice — lesen, ablegen, publizieren — sind fehlbar.** Jeder
  gibt ein `Result` mit dem **eigenen, redigierten, terminalen Slice-Fehler** zurück.
- **Jeder Fehlschlag ist terminal.** Quelle nicht lesbar, Ablage scheitert, Publish scheitert
  — alles terminal. Es gibt **kein Dead-Letter-Konzept und kein Retry im Slice**; ein
  Domänen-„unavailable"/„absent" bedeutet **permanente** Unerreichbarkeit → terminal. Der
  Handler-Fehlerkanal ist damit **luftdicht uniform-terminal**.
- **Der Slice gibt seinen nackten, redigierten Domänen-Fehler zurück** und baut **keinen**
  Transport-/Klassifikations-Fehlertyp. Die frühere `io`-Klassifikation
  (`Warning`/`Retry`/`Critical`) und die attempt-basierte Eskalation → Dead-Letter
  **entfallen** als Missverständnis. **Wie `io` einen terminalen Fehlschlag surfacet, ist
  vertagt** (entscheidet der `io`-Schritt) — nicht in diesem Vertrag vorgeschrieben.
- **Redaktion gewahrt** (ADR-0008-Datenschutz): der Slice-Fehler ist alleiniger Eigentümer
  seines `Display`-Satzes — nur **statischer Feldname**, opake ID oder io-Diagnose, **nie ein
  Rohwert**.

## Entscheidung B — uniformes ID-Handoff

- **Jeder Slice** liest die **Vorgänger-Zeile per ID**, baut sein Aggregat, transformiert,
  schreibt in seine **eigene** neue Tabelle — **die Persistenz vergibt die neue ID** — und
  **publiziert diese neue ID** als Zeiger auf den nächsten Schritt. Der Handler wird eine
  geradlinige `?`-Kette: **lesen → transformieren → ablegen (neue ID) → neue ID publizieren**,
  ohne Übersetzungs-`match`.
- **`einlesen` ist der Kopf:** seine Quelle ist eine **Datei hinter dem Seam**, per
  **Auslöser** getriggert (feldloser Trigger-Command), **nicht** per Vorgänger-ID. Ab hier
  ist das Handoff uniform.
- Die **Ablage ist fehlbar und gibt die neu vergebene ID zurück**; **nicht** die gelesene
  Quell-ID reist weiter, sondern die **neue Identität der eigenen Zeile**.

## Entscheidung C — Aggregate referenzieren Fremd-Entitäten nur per ID

- Ein Aggregat referenziert eine **Fremd-Entität ausschließlich per ID** (Fremdschlüssel),
  **nie durch ein eingebettetes fremdes Objekt**. Beispiel: die obfuskierte Buchung
  referenziert ihre Quell-Buchung **nur per ID** — weder als publizierter Zeiger (das ist die
  neu vergebene eigene ID, Entscheidung B) noch als eingebettetes Roh-Objekt.
- Das hält die Slice-Grenze sauber (kein fremder Domänen-Typ überquert sie) und stützt die
  Datenschutz-Grenze: ein Downstream-Aggregat kann keinen rohen Fremd-Zustand mitschleppen,
  den es gar nicht eingebettet hält.

## Konsequenzen

- **Einfachere Handler:** eine `?`-Kette statt eines Übersetzungs-`match`; kein
  `HandleOutcome`/`Severity`/`OutboxError`-Bau im Slice, kein spiegelbildlicher
  Handler-Fehler zum Lese-Ergebnis.
- **Kein geteilter Fehler-Crate:** jeder Slice trägt seinen **eigenen** terminalen
  `SourceError` (Namensgleichheit zwischen Slices ist akzeptiert), redigiert und ohne Rohwert.
- **Die Retry/Dead-Letter-Maschinerie verlässt das Slice-Modell vollständig.** Was `io` mit
  einem terminalen Fehlschlag tut (protokollieren, in eine Quarantäne-Tabelle legen,
  abbrechen) und ob es intern — hinter seinen eigenen Lese-Seams oder auf Broker-Ebene —
  wiederholt, entscheidet der `io`-Schritt; der Slice-Vertrag schreibt es **nicht** vor.

## Was dieses ADR supersedet / verfeinert

- **ADR-0011, Entscheidung B (Fehler-Taktik):** „jeder Handler-Fehler ist terminal →
  **Dead-Letter**" plus die `io`-Klassifikation `Warning`/`Retry`/`Critical` und die
  attempt-basierte Eskalation werden durch **„durchweg terminal, kein Dead-Letter, kein
  Retry im Slice"** ersetzt. Dass der Slice seinen **nackten, redigierten** Domänen-Fehler
  zurückgibt, bleibt. Ob und wie `io` einen terminalen Fehlschlag surfacet oder intern
  wiederholt, ist **vertagt** (io-Schritt) — **nicht** mehr als `io`-Klassifikations-Vertrag
  vorgeschrieben. Entscheidung A/C/D von ADR-0011 (Slice = reine Pipeline; Outbox-Mechanik +
  Kernel-Flip in `io`) bleiben **maßgeblich**.
- **ADR-0008, § „Der Handler arbeitet auf Domänen-Typen" + Envelope-Konvention:** das
  ID-Handoff wird **uniform** festgeschrieben — die **von der Ablage neu vergebene ID** ist
  der **publizierte Zeiger** (nicht die gelesene Quell-ID), und Aggregate referenzieren
  Fremd-Entitäten **nur per ID**. Dass der Aggregat-Zeiger im `partition_key` reist und ein
  ID-Zeiger-Consumer den Vorgänger per Aggregat-ID auflöst, bleibt maßgeblich.
- **ADR-0008, Pipeline-Garantien** (atomar, at-least-once, wiederaufsetzbar) und die
  **Datenschutz-Grenze an `obfuskieren`** bleiben **unberührt maßgeblich**.

Die Umsetzung dieses Musters in den Slices ist **bereits erfolgt** — die Slice-Issues
[0028](../issues/0028-obfuskierung-terminal-fehlbares-read-save-publish-und-id-handoff.md)
(`obfuskierung`) und
[0029](../issues/0029-einlesen-terminal-fehlbares-store-publish.md) (`einlesen`) sind
umgesetzt: lesen/ablegen/publizieren je fehlbar+terminal, die Ablage vergibt die neue ID, genau sie
reist als Zeiger, Fremd-Referenz nur per ID. Dieses ADR ist der **kanonische Ort der
Entscheidung**, keine Code-Änderung selbst.

## Bewusst vertagt

- **Wie `io` einen terminalen Fehlschlag surfacet** (Log, Quarantäne-Tabelle, Abbruch) und
  ob `io` intern wiederholt — entscheidet der `io`-Schritt, wenn die echte Outbox/DB gebaut
  wird.
- **Die Angleichung des `io`-Outbox-Consumers** an dieses ADR: die heutige
  `io`-Teil-/Placeholder-Implementierung klassifiziert einen Handler-Fehler noch nach dem
  **alten** 0011-Modell (`Warning`/`Retry`/`Critical`); die Umstellung auf „durchweg terminal"
  geschieht mit dem echten `io`-Crate (die Slices selbst geben schon nur ihren nackten,
  terminalen Domänen-Fehler zurück).
