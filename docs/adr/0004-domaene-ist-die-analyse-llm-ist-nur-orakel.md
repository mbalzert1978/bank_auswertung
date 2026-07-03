# Die Domäne ist die Analyse — das LLM ist nur ein Orakel für unbekannte Fingerabdrücke

> **Status: in Teilen superseded — durch [ADR-0007](0007-modularer-monolith-feature-slices.md), [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md).**
> Der Textkörper unten ist **Historie** (Entscheidungsstand zum ADR-Zeitpunkt) und
> bleibt unverändert. Was heute gilt und was nicht mehr, steht **kanonisch** im
> Abschnitt „Was dieses ADR supersedet" von [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) (bzw. [ADR-0007](0007-modularer-monolith-feature-slices.md)).

Lesen, Obfuskieren und Präsentieren sind Adapter-Aufgaben (rein/raus), **nicht**
die Domäne. Die Domäne ist die Finanzanalyse und lebt rein und testbar im Kern:
Wiederkehrenden Abfluss erkennen (inkl. Jahreskosten und Preissteigerung),
Umbuchungen netten (die Domäne matcht auf `Gegenkonto::Eigen`; _welche_ Gegen-IBAN
eine eigene ist, hat der eingehende Adapter bereits aufgelöst), Gebühren
klassifizieren und summieren, Aderlass/Cashflow je Konto und Monat rechnen,
Pareto-Ranking der Treiber, Spar-Kandidaten strukturell finden, und das
Kategorie-Register mit gelernten Korrekturen besitzen.

Das LLM ist **kein** Kategorisierer-of-record, sondern ein Orakel hinter dem
`Kategorisierer`-Port: Es bildet nur die der Domäne **unbekannten** Fingerabdrücke
vom Freitext auf eine Kategorie ab. Den Fingerabdruck selbst bildet die Domäne —
als reine Funktion `Buchung → Fingerabdruck`, inklusive Zahlungsintermediär-
Erkennung und Händler-Token; diese Logik ist Domänenwissen und gehört **nicht**
in einen Adapter. Die Domäne entscheidet selbst, was bekannt ist und was sie
überhaupt fragen muss.

## Lackmustest

Entfernt man das LLM ganz, findet das Programm weiterhin jeden wiederkehrenden
Abfluss, nettet Umbuchungen, summiert Gebühren, rankt die größten Abflüsse und
zeigt den Trend — es fehlt nur das automatische Etikett für unbekannte Händler.
Wenn das Programm ohne LLM noch wertvoll ist, ist die Domäne echt.

## Konsequenzen

- Die HTML-Ausgabe zeigt die **Analyse der Domäne**, nicht „was das LLM sagte".
  Die Kategorie ist nur ein Attribut einer `Buchung`.
- Das LLM wird nur für offene Fingerabdrücke aufgerufen — das hält API-Kosten
  und Datenabfluss minimal.
