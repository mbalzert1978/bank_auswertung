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
    amount_cent: u64,
    currency: NonEmptyString,
    direction: NonEmptyString,
    booking_day: NonEmptyString,
    value_date: NonEmptyString,
}

impl PassThrough {
    /// Setzt die durchgereichten Felder aus ihren bereits an der Adapter-Grenze (ACL) geprüften
    /// Teilen zusammen — der Adapter reicht sie hier hinein und gießt das Ganze weiter in
    /// [`IngestedBooking::hydrate`](crate::IngestedBooking::hydrate). Der Betrag ist bereits ein
    /// dekodierter Cent-`u64`, die übrigen sind [`NonEmptyString`].
    #[must_use]
    pub fn from_parts(
        amount_cent: u64,
        currency: NonEmptyString,
        direction: NonEmptyString,
        booking_day: NonEmptyString,
        value_date: NonEmptyString,
    ) -> Self {
        Self {
            amount_cent,
            currency,
            direction,
            booking_day,
            value_date,
        }
    }

    #[must_use]
    pub fn amount_cent(&self) -> u64 {
        self.amount_cent
    }

    #[must_use]
    pub fn currency(&self) -> &NonEmptyString {
        &self.currency
    }

    #[must_use]
    pub fn direction(&self) -> &NonEmptyString {
        &self.direction
    }

    #[must_use]
    pub fn booking_day(&self) -> &NonEmptyString {
        &self.booking_day
    }

    #[must_use]
    pub fn value_date(&self) -> &NonEmptyString {
        &self.value_date
    }
}
