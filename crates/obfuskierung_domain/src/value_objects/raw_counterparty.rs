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
    pub ordering_account: NonEmptyString,
    pub counter_account: Option<NonEmptyString>,
    pub recipient_name: Option<NonEmptyString>,
    pub creditor_id: Option<NonEmptyString>,
    pub purpose: Option<NonEmptyString>,
    pub booking_text: Option<NonEmptyString>,
}
