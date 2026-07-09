use crate::value_objects::{DecodedAmount, NonEmptyString};

/// Der von der Domäne **verlangte** Port, der das rohe Betragsfeld zu einem fertig
/// dekodierten [`DecodedAmount`] deutet (vorzeichenlose Cent-Menge + zwei-wertiges
/// Vorzeichen). Die Domäne **validiert nicht** — sie fordert den fertigen Wert an und
/// bleibt dependency- und formats-blind; die *Zahlenformat*-Dekodierung (deutscher
/// Tausenderpunkt, Dezimalkomma, Nachkommastellen, Erkennen/Abstreifen des führenden
/// Vorzeichens) ist Mechanik, keine fachliche Entscheidung, und lebt hinter diesem Port
/// (ADR-0008 — wie das Datum hinter dem `Calendar`-Port); die Tests unterschieben einen
/// Fake. Der Vertrag ist wert-typisiert: die Domäne fordert einen `NonEmptyString` an,
/// **kein** primitives `&str`, und bekommt ein VO zurück, nie ein nacktes Tupel.
pub trait Money {
    /// Dekodiert das rohe Betragsfeld zum [`DecodedAmount`], oder `None`, wenn es kein
    /// gültiger Betrag ist — auch ein fehlendes oder undeutbares Vorzeichen ist ein
    /// solches `None` (das Aggregat macht daraus `AmountInvalid`). Die Domäne sieht nie
    /// ein „halb dekodiertes" Feld.
    fn decode(&self, raw: &NonEmptyString) -> Option<DecodedAmount>;
}
