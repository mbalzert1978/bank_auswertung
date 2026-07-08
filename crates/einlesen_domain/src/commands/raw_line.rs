use crate::value_objects::NonEmptyString;

/// Eine **rohe Buchungszeile als Domänen-Wert** — das Ergebnis der Adapter-Abbildung
/// „außen → innen": das Wire-DTO `RawRecord` wird im Adapter (ACL) auf diesen Domänen-VO
/// gemappt, sodass jede Zelle bereits als `NonEmptyString` am Domänen-Rand geprüft vorliegt
/// (leer/fehlend = `None`, an *einem* Ort abgewiesen). Das Kommando
/// [`ReadTransactions`](crate::ReadTransactions) trägt einen Batch davon; das
/// [`LineAggregate`](crate::LineAggregate) deutet eine `RawLine` weiter zu reichen
/// Wert-Objekten — die Domäne spricht nie ein primitives `String`.
#[derive(Debug, Clone)]
pub struct RawLine {
    pub ordering_account: Option<NonEmptyString>,
    pub booking_day: Option<NonEmptyString>,
    pub value_date: Option<NonEmptyString>,
    pub amount: Option<NonEmptyString>,
    pub currency: Option<NonEmptyString>,
    pub recipient_name: Option<NonEmptyString>,
    pub creditor_id: Option<NonEmptyString>,
    pub counter_account: Option<NonEmptyString>,
    pub purpose: Option<NonEmptyString>,
    pub booking_text: Option<NonEmptyString>,
}
