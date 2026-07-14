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
    /// **Hier erzwingt der Adapter die Präsenz** der Pflichtfelder (Auftraggeberkonto, Währung,
    /// Richtung, Datum): fehlt/ist eines leer, wird es zu `Err(MissingMandatoryField)` mit **nur**
    /// dem Feldnamen — ein terminaler Ausgang, den die [`SourceBridge`](crate::bridges::SourceBridge)
    /// von „nicht vorhanden" (`Absent`) unterscheidet: **beide** terminal, je eigener redigierter
    /// Grund (opake ID vs. statischer Feldname).
    pub(crate) fn to_booking(
        &self,
        id: &BookingId,
    ) -> Result<IngestedBooking, MissingMandatoryField> {
        let counterparty = RawCounterparty {
            ordering_account: mandatory(&self.ordering_account, "ordering_account")?,
            counter_account: as_nonempty(&self.counter_account),
            recipient_name: as_nonempty(&self.recipient_name),
            creditor_id: as_nonempty(&self.creditor_id),
            purpose: as_nonempty(&self.purpose),
            booking_text: as_nonempty(&self.booking_text),
        };
        let pass_through = PassThrough {
            amount_cent: self.amount_cent,
            currency: mandatory(&self.currency, "currency")?,
            direction: mandatory(&self.direction, "direction")?,
            booking_day: mandatory(&self.booking_day, "booking_day")?,
            value_date: mandatory(&self.value_date, "value_date")?,
        };
        Ok(IngestedBooking::new(id.clone(), counterparty, pass_through))
    }
}

/// Ein Pflichtfeld fehlt (leer/nicht vorhanden) in der Roh-Zeile — **terminaler** Ausgang der
/// „außen → innen"-Abbildung (die [`SourceBridge`](crate::bridges::SourceBridge) mappt ihn auf ein
/// Dead-Letter). **Datenschutz (Redaktions-Invariante):** trägt **nur** den statischen Feldnamen,
/// nie den Rohwert — auch `Debug` spiegelt so keinen Identifikator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingMandatoryField {
    field: &'static str,
}

impl MissingMandatoryField {
    /// Der (statische, unsensible) Name des fehlenden Pflichtfelds — für den redigierten
    /// Dead-Letter-Grund.
    pub(crate) fn field(&self) -> &'static str {
        self.field
    }
}

/// Erzwingt ein Pflichtfeld: reicht die Zelle als `NonEmptyString` an die Domänengrenze, oder —
/// fehlt/leer sie — den redigierten [`MissingMandatoryField`] mit nur dem Feldnamen.
fn mandatory(raw: &str, field: &'static str) -> Result<NonEmptyString, MissingMandatoryField> {
    NonEmptyString::new(raw).ok_or(MissingMandatoryField { field })
}

/// Reicht eine optionale rohe Zelle als `NonEmptyString` an die Domänengrenze (leer/`None` →
/// `None`); der `NonEmptyString`-Kern trimmt und weist Leeres genau hier ab.
fn as_nonempty(raw: &Option<String>) -> Option<NonEmptyString> {
    raw.as_deref().and_then(NonEmptyString::new)
}
