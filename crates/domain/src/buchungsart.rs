/// Die maschinelle Art einer Buchung (Treiber der deterministischen
/// Wiederholungs- und Gebühren-Erkennung). `Sonstige` fängt Buchungstexte ab,
/// die noch keiner der bekannten Arten zugeordnet sind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buchungsart {
    Lastschrift,
    Dauerauftrag,
    Überweisung,
    Entgelt,
    Sonstige,
}
