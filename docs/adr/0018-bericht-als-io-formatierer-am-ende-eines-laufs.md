# Der Bericht ist ein `io`-Formatierer am Ende eines Exit-0-Laufs

> **Status: Akzeptiert, gebaut** (Issue [0012](../issues/0012-html-bericht.md)). Verfeinert die
> „report"-Aussagen von [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) (und damit die
> Pipeline-Zeile von [ADR-0013](0013-zeitfenster-collector-paginierender-analyse-kopf.md)).

## Kontext

ADR-0008 zeichnete die Pipeline als `einlesen → obfuskieren → auswerten → report`: „**report** —
konsumiert das Endergebnis → HTML", und vertagte eine eigene `report`-Presenter-Crate. Als der
Bericht gebaut wurde, stand fest:

- Die fenster-übergreifende Reichweite von `auswerten` **gibt ihr Ergebnis zurück, statt es
  abzulegen** (ADR-0013, Entscheidung E). Es gibt keine Nachricht, die ein `report`-Consumer
  konsumieren könnte, und keine Tabelle, aus der er das Ergebnis lesen könnte.
- Der Bericht gehört zu **genau einem** Lauf: am Ende eines Laufs mit Exit 0, für die `run_id`, die
  dieser Lauf gemintet hat — nie `max(run_id)`, nach Exit 1 gar nicht (ADR-0016, ADR-0017).
- Der Bericht **rechnet nichts**. Jede Zahl liefert die Domäne (`HistoryEvaluation`, samt
  Umbuchungen und Gebühren über den ganzen Lauf). Was bleibt, ist Beschriften, Ordnen, Formatieren
  und Escapen — Format-Wissen.

## Entscheidung

Der Bericht ist ein **Formatierer in `io`** (`crates/io/src/report/`), den `io::run` am Ende eines
gelungenen Laufs aufruft:

- **Kein Outbox-Consumer.** Der Lauf hält das Ergebnis der fenster-übergreifenden Reichweite in
  seinem Abnehmer fest und reicht es nach dem letzten Drain an den Bericht. Ein Consumer bräuchte
  eine Nachricht mit dem Ergebnis (oder eine abgelegte Kopie) — beides gibt es bewusst nicht, und eine
  Nachricht „Lauf fertig" wäre erst nach dem Drain sinnvoll, also ohnehin außerhalb der Pipeline.
- **Kein eigener Port.** Es gibt genau ein Ausgabeformat. Ein `Bericht`-Trait käme mit dem zweiten.
- **Keine eigene Crate oder Slice-Triade.** Eine `report_domain` hätte keine Domänen-Regel zu tragen;
  HTML gehört wie das CSV-Format beim Einlesen nach `io`.
- **Egress bleibt compilerfest:** das Anzeige-DTO trägt `assert_not_impl_any!(…: Serialize)`. Konten
  erscheinen über ihre Rolle, Personen nur pseudonym, Firmen über ihren gescrubbten Namen (nur wenn
  er keinen Namen der lokalen Namensliste nennt) oder ihre Gläubiger-ID; jeder Text wird escaped.
- Geschrieben wird in eine Nachbar-Datei, die nach `sync_all` auf das Ziel umbenannt wird; nach
  Exit 1 bleibt eine ältere Datei unberührt.

## Konsequenzen

- Die Pipeline-Zeile lautet weiter `einlesen → obfuskieren → fenstern → auswerten → report`, aber
  `report` ist **kein Schritt der Outbox-Kette**, sondern der Abschluss des Laufs.
- Ein Bericht über einen früheren Lauf lässt sich nicht nachträglich erzeugen — das Ergebnis wird
  nicht abgelegt. Gebraucht wird das nicht: derselbe Bestand in einem neuen Lauf ergibt dieselben
  Zahlen (ADR-0016).
- Sollte je ein zweites Format (PDF, Chat-Export) dazukommen, ist der Moment für einen Port da; die
  Zusammensetzung des Anzeige-DTO ist schon eine reine Funktion und bleibt.
