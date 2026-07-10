/// Die **Identität**, die der Slice einem eingelesenen Aggregat vergibt — der je abgelegter
/// Roh-Zeile eindeutige Schlüssel, unter dem die Roh-Erfassung persistiert und den der Slice als
/// **Zeiger** publiziert (Tabelle = Wahrheit,
/// Message = Pointer,
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)).
/// Der nachgelagerte `obfuskieren`-Slice lädt das Aggregat später **per dieser ID** aus dem
/// einlesen-Speicher. Opak — **kein** struktureller Identifikator, nie ein Rohwert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IngestionId(String);

impl IngestionId {
    /// Baut die ID aus ihrem Rohwert (der [`Store`](crate::Store)-Port vergibt sie beim Ablegen und
    /// reicht sie herein). Bewusst **opak** (kein `NonEmptyString`): die Nicht-Leere garantiert die
    /// Herkunft (io/der Speicher vergibt die ID, nie leer) — ein eigener Validierungspfad wäre
    /// reibungsvolle Zeremonie ohne Gewinn, denn der Slice deutet die ID nie, er reicht sie durch.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
