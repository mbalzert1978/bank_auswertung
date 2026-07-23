use crate::value_objects::{Currency, DecodedAmount, NonEmptyString};

/// Der von der Domäne **verlangte** Port, der das rohe Betragsfeld **für die gegebene Währung**
/// zu einem fertig dekodierten [`DecodedAmount`] deutet (vorzeichenlose Menge in **Minor Units**
/// samt zwei-wertigem Vorzeichen). Die Domäne **validiert nicht** — sie fordert den fertigen Wert
/// an und bleibt dependency- und formats-blind; die *Zahlenformat*-Dekodierung (deutscher
/// Tausenderpunkt, Dezimalkomma, Nachkommastellen, Erkennen/Abstreifen des führenden Vorzeichens)
/// **und** die währungs-bewusste Skalierung auf die Minor Units **dieser** Währung (ISO 4217)
/// sind Mechanik, keine fachliche Entscheidung, und leben hinter diesem Port (ADR-0008 — wie das
/// Datum hinter dem `Calendar`-Port); die Tests unterschieben einen Fake. Die Domäne reicht ihre
/// bereits gedeutete [`Currency`] herein — sie bleibt **Wahrheit für die Währung**, die Minor
/// Unit liefert `io`.
pub trait Money {
    /// Dekodiert das rohe Betragsfeld zum [`DecodedAmount`] in den Minor Units von `currency`,
    /// oder `None`, wenn es kein gültiger Betrag ist — ein fehlendes/undeutbares Vorzeichen oder
    /// ein für die Währung zu genauer Wert (mehr Nachkommastellen als ihre Minor Unit) ist ein
    /// solches `None` (das Aggregat macht daraus `AmountInvalid`). Die Domäne sieht nie ein „halb
    /// dekodiertes" Feld. Die [`Currency`] reist **per Referenz** herein — sie trägt die Minor Unit,
    /// nach der skaliert wird, und wird danach unverändert in den Betrag bewegt.
    fn decode(&self, raw: &NonEmptyString, currency: &Currency) -> Option<DecodedAmount>;
}
