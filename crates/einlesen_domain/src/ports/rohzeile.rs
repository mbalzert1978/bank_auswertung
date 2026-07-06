use crate::value_objects::NonEmptyString;

/// Der von der Domäne **verlangte** Port für die Rohdaten *einer* Zeile. Die Domäne
/// fordert nur an, **welche** Felder sie braucht (jeweils als optionaler
/// `NonEmptyString`); **wie** eine Rohzeile adressiert und gebaut wird (CSV-Spalte,
/// Reihenfolge, Header-Zuordnung), ist Sache des Adapters, der dieses Interface erfüllt.
/// Jedes Feld liefert ein `Option`, weil die Pflicht/Optional-Entscheidung erst im
/// `ZeilenAggregat` fällt (an einem Ort); ein fehlendes oder leeres Feld ist `None`.
pub trait Rohzeile {
    fn auftragskonto(&self) -> Option<NonEmptyString>;
    fn buchungstag(&self) -> Option<NonEmptyString>;
    fn valuta(&self) -> Option<NonEmptyString>;
    fn betrag(&self) -> Option<NonEmptyString>;
    fn währung(&self) -> Option<NonEmptyString>;
    fn empfänger_name(&self) -> Option<NonEmptyString>;
    fn gläubiger_id(&self) -> Option<NonEmptyString>;
    fn gegenkonto(&self) -> Option<NonEmptyString>;
    fn verwendungszweck(&self) -> Option<NonEmptyString>;
    fn buchungstext(&self) -> Option<NonEmptyString>;
}
