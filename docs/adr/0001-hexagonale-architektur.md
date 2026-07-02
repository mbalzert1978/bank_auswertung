# Hexagonale Architektur — die Domäne kennt keine Dateiformate und keinen LLM-Anbieter

Wir bauen wartbare Software, kein Skript. Die Domäne arbeitet ausschließlich mit
dem Domänenmodell `Buchung` und kennt weder Exportformate noch konkrete externe
Dienste. Jede externe Information wird hinter einem Port (Interface) abstrahiert.

**Eingehend** — Port `Umsatzquelle`: Adapter parsen je ein Format in `Buchung`-
Objekte (CSV-CAMT-Adapter zuerst; XML-camt und MT940 sind später nur weitere
Adapter hinter demselben Port, ohne Domänenänderung).

**Ausgehend** — Port `Kategorisierer`: Auch das LLM ist „externe Information" und
sitzt damit hinter einem Interface. Die Domäne sagt „kategorisiere diese
Buchungen", nicht „ruf Claude". Dahinter kann ein LLM, ein Regelwerk oder ein
Test-Fake stehen.

**Ausgehend** — Port `Bericht`: Die Ausgabe der Auswertung ist austauschbar.
Erster Adapter rendert schönes HTML; Tabelle/CLI/andere Formate sind später nur
weitere Adapter hinter demselben Port.

**Domäne** (rein, kein I/O): Wiederkehrende-Abfluss-Erkennung, Gebühren-
Markierung, Umbuchungs-Erkennung, Aggregation — arbeitet nur auf `Buchung`, voll
testbar ohne Datei und ohne Netz.

## Konsequenzen

- **Obfuskierung lebt am eingehenden Adapter** — er löst rohe Identifikatoren
  (IBAN/BIC, Auftragskonto, Privatname) beim Mappen in obfuskierte Value Objects
  und typisierte Tatsachen auf (`Gegenkonto::Eigen|Fremd`,
  `Empfänger::Firma|Person`, `KONTO_x`); Freitext wird zu `ObfuskierterText`
  gescrubbt. Die Domäne erhält damit **keinen personen- oder kontobeziehbaren**
  Identifikator; sie hält nur obfuskierte Werte, die Klartext-Zuordnung bleibt
  lokal im Adapter. Bewusste Ausnahme: die Gläubiger-ID (öffentlicher
  Firmen-Schlüssel, nicht personenbezogen) bleibt als Analyse-Anker erhalten.
  Der `Kategorisierer`-Adapter kann nichts strukturell Rohes weitergeben
  (compilergeprüft, [ADR-0003](0003-rust-als-sprache.md)).
- Ein weiteres Eingabeformat kostet einen Adapter, keine Domänenänderung.
