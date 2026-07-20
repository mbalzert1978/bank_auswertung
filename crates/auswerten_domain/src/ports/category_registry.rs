use crate::error::EvaluationError;
use crate::value_objects::Fingerprint;

/// Der von der Domäne **verlangte** Lese-Port des **Kategorie-Registers**: er beantwortet die eine
/// Frage, die „unbekannt" überhaupt erst definiert — **welche dieser Buchungsarten kenne ich
/// schon?** (ADR-0004: „die Domäne entscheidet selbst, was bekannt ist und was sie überhaupt fragen
/// muss").
///
/// **Eine Filter-Frage, kein „gib mir das ganze Register":** der Port bekommt die Kandidaten des
/// Fensters herein und gibt die bekannten davon zurück. Das hält die Frage auf das gescopt, was
/// gerade ausgewertet wird — ein Port, der das vollständige Register herausreichte, zwänge io, bei
/// jedem Fenster alles Gelernte zu materialisieren, und wüchse mit der Historie statt mit dem
/// Fenster.
///
/// **Er reicht die Schlüssel zurück, die er bekam** — nicht frisch gebildete. Der Fingerabdruck
/// entsteht **nur** als reine Domänenfunktion aus einer Gegenpartei
/// ([`Fingerprint`](crate::Fingerprint)); es gibt bewusst keinen Weg, einen aus einer rohen
/// Zeichenkette zu erzeugen. Sonst könnte ein Adapter einen Schlüssel *erfinden*, und der
/// Fingerabdruck wäre nicht mehr Domänenwissen, sondern eine Absprache über Strings.
///
/// **Fehlbar und terminal** (ADR-0012): scheitert die Auskunft, endet der Lauf. Sie zu überspringen
/// hieße, alles für unbekannt zu halten und das Orakel mit bereits Gelerntem zu behelligen — der
/// stille Fehler, den dieser Slice nicht macht.
pub trait CategoryRegistry {
    /// Gibt die Teilmenge der Kandidaten zurück, die das Register bereits kennt.
    fn known_among(&self, candidates: &[Fingerprint]) -> Result<Vec<Fingerprint>, EvaluationError>;
}
