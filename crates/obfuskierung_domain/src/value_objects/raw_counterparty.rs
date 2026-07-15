use crate::value_objects::non_empty_string::NonEmptyString;

/// Die **rohe** Gegenpartei-Sicht einer eingelesenen Zeile, wie der Adapter sie „außen → innen"
/// an die Domäne reicht (ACL): nur die Felder, die die Obfuskierung tatsächlich anfasst —
/// Identifikatoren (Konten) und das Kategorie-Signal (Empfänger-Name, Gläubiger-ID) plus der
/// Freitext. Betrag/Währung/Richtung/Datum reist **nicht** hier durch (kein Obfuskierungs-Belang;
/// der Adapter reicht es verbatim um die Domäne herum).
///
/// `ordering_account` ist Pflicht (der Adapter erzwingt seine Präsenz vor dem Bau); die übrigen
/// sind genuin optional — eine Gebührenzeile hat keine Gegenpartei, eine Person keine
/// Gläubiger-ID. Alle Werte sind bereits als [`NonEmptyString`] geprüft (parse, don't validate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCounterparty {
    pub(crate) ordering_account: NonEmptyString,
    pub(crate) counter_account: Option<NonEmptyString>,
    pub(crate) recipient_name: Option<NonEmptyString>,
    pub(crate) creditor_id: Option<NonEmptyString>,
    pub(crate) purpose: Option<NonEmptyString>,
    pub(crate) booking_text: Option<NonEmptyString>,
}

impl RawCounterparty {
    /// Setzt die rohe Gegenpartei-Sicht aus ihren bereits an der Adapter-Grenze (ACL) geprüften
    /// Teilen zusammen — der Adapter reicht die von ihm gebauten [`NonEmptyString`]-Werte hier hinein
    /// und gießt das Ganze weiter in [`IngestedBooking::hydrate`](crate::IngestedBooking::hydrate).
    #[must_use]
    pub fn from_parts(
        ordering_account: NonEmptyString,
        counter_account: Option<NonEmptyString>,
        recipient_name: Option<NonEmptyString>,
        creditor_id: Option<NonEmptyString>,
        purpose: Option<NonEmptyString>,
        booking_text: Option<NonEmptyString>,
    ) -> Self {
        Self {
            ordering_account,
            counter_account,
            recipient_name,
            creditor_id,
            purpose,
            booking_text,
        }
    }
}
