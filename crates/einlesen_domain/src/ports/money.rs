use crate::value_objects::{NonEmptyString, Sign};

/// Der von der Domäne **verlangte** Port, der das rohe Betragsfeld zu seinen
/// Bestandteilen dekodiert: vorzeichenlose Cent-Menge plus drei-wertiges Vorzeichen. Die
/// Domäne bleibt dependency-frei und formats-blind; die *Zahlenformat*-Dekodierung
/// (deutscher Tausenderpunkt, Dezimalkomma, genau zwei Nachkommastellen, Gruppierung,
/// Erkennen/Abstreifen des führenden Vorzeichens) ist Mechanik, keine fachliche
/// Entscheidung, und lebt hinter diesem Port (ADR-0008 — wie das Datum hinter dem
/// `Calendar`-Port); die Tests unterschieben einen Fake. Der Vertrag ist wert-typisiert:
/// die Domäne fordert einen `NonEmptyString` an, **kein** primitives `&str`. Der Port
/// trägt **nur** die eine Fehlbedeutung „kein gültiger Betrag" als `None`; die
/// **fachliche** Deutung der Bestandteile — Vorzeichen → `Direction`, fehlendes
/// Vorzeichen → `SignMissing` — bleibt in der Domäne (`Amount::with_direction`).
pub trait Money {
    /// Dekodiert das rohe Betragsfeld zur vorzeichenlosen Cent-Menge samt drei-wertigem
    /// Vorzeichen, oder `None`, wenn es kein gültiger Betrag ist. Ein fehlendes
    /// Vorzeichen ist **kein** Dekodierfehler, sondern der Fall `Sign::Missing` — nie
    /// still positiv.
    fn decode(&self, raw: &NonEmptyString) -> Option<(u64, Sign)>;
}
