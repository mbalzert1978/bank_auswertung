/// Die **Identität** eines eingelesenen Aggregats — der Schlüssel, unter dem der Slice das
/// obfuskierte Aggregat ablegt und den er als **Zeiger** publiziert (Tabelle = Wahrheit, Message =
/// Pointer). Aktuell die opake Envelope-ID der eingehenden Nachricht; ab
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)
/// die von `einlesen` vergebene Aggregat-ID, über die der Handler das Aggregat aus dem
/// einlesen-Speicher lädt. Opak — kein struktureller Identifikator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BookingId(String);

impl BookingId {
    /// Baut die ID aus ihrem Rohwert (der Adapter reicht die Envelope-ID herein). Bewusst **opak**
    /// (kein `NonEmptyString`): die Nicht-Leere garantiert die Herkunft (io/Outbox-Envelope vergeben
    /// die ID, nie leer) — ein eigener Validierungspfad hier wäre reibungsvolle Zeremonie ohne
    /// Gewinn, denn der Slice deutet die ID nie, er reicht sie als Zeiger durch.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
