use obfuskierung_domain::{
    BookingId, IngestedBooking, NonEmptyString, PassThrough, RawCounterparty,
};

/// Die **rohe einlesen-Zeile**, wie sie der Lese-Seam aus dem einlesen-Speicher (Roh-Tabelle)
/// liefert — die zehn verwerteten Felder als rohe Zellen. Reines Zwischen-DTO zwischen dem Seam
/// (io) und der Domäne; die Abbildung „außen → innen" (Zeile → Domänen-Aggregat [`IngestedBooking`])
/// lebt hier (ACL). **Kein serde**: io baut die Zeile aus dem Speicher, sie wird nicht aus einer
/// Nachricht deserialisiert (die Nachricht trägt nur die ID).
#[derive(Debug, Clone, Default)]
pub struct IngestedRow {
    pub ordering_account: String,
    pub counter_account: Option<String>,
    pub amount_cent: u64,
    pub currency: String,
    pub direction: String,
    pub recipient_name: Option<String>,
    pub creditor_id: Option<String>,
    pub booking_day: String,
    pub value_date: String,
    pub purpose: Option<String>,
    pub booking_text: Option<String>,
}

impl IngestedRow {
    /// Deutet die rohe Zeile in das Domänen-Aggregat [`IngestedBooking`] (ACL, „außen → innen").
    /// `None`, wenn ein Pflichtfeld (Auftraggeberkonto, Währung, Richtung, Datum) leer ist — dann
    /// ist die Zeile keine verwertbare Buchung; der Adapter behandelt das wie „nicht vorhanden".
    pub(crate) fn to_booking(&self, id: BookingId) -> Option<IngestedBooking> {
        let counterparty = RawCounterparty {
            ordering_account: NonEmptyString::new(&self.ordering_account)?,
            counter_account: as_nonempty(&self.counter_account),
            recipient_name: as_nonempty(&self.recipient_name),
            creditor_id: as_nonempty(&self.creditor_id),
            purpose: as_nonempty(&self.purpose),
            booking_text: as_nonempty(&self.booking_text),
        };
        let pass_through = PassThrough {
            amount_cent: self.amount_cent,
            currency: NonEmptyString::new(&self.currency)?,
            direction: NonEmptyString::new(&self.direction)?,
            booking_day: NonEmptyString::new(&self.booking_day)?,
            value_date: NonEmptyString::new(&self.value_date)?,
        };
        Some(IngestedBooking::new(id, counterparty, pass_through))
    }
}

/// Reicht eine optionale rohe Zelle als `NonEmptyString` an die Domänengrenze (leer/`None` →
/// `None`); der `NonEmptyString`-Kern trimmt und weist Leeres genau hier ab.
fn as_nonempty(raw: &Option<String>) -> Option<NonEmptyString> {
    raw.as_deref().and_then(NonEmptyString::new)
}
