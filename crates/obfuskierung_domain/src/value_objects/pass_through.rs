use crate::value_objects::non_empty_string::NonEmptyString;

/// Die **nicht-obfuskierten** Felder einer Buchung, die der Slice unverändert **mitträgt**:
/// Betrag, Währung, Richtung und die beiden Datumsangaben. Die Obfuskierung fasst sie nicht an —
/// sie sind keine Identifikatoren —, aber das obfuskierte Aggregat muss sie führen, damit
/// stromabwärts (`auswerten`) die vollständige Buchung vorliegt.
///
/// Bewusst **opak getragen**: dieser Slice **deutet** die Felder nicht (das ist Sache von
/// `auswerten`), er reicht die bereits von `einlesen` dekodierten Werte durch — als geprüfte
/// [`NonEmptyString`] statt roher `String` (parse, don't validate). Der Betrag steht als Cent-`u64`
/// (nicht als `NonEmptyString`) bewusst uneinheitlich da: er ist bereits ein **dekodierter Zahlwert**
/// (kein Textfeld, auf das „nicht-leer" zuträfe) und wird ebenso opak durchgereicht — `auswerten`
/// deutet ihn, dieser Slice fasst ihn nicht an.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassThrough {
    pub amount_cent: u64,
    pub currency: NonEmptyString,
    pub direction: NonEmptyString,
    pub booking_day: NonEmptyString,
    pub value_date: NonEmptyString,
}
