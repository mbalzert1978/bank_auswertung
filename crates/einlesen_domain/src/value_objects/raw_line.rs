use crate::value_objects::NonEmptyString;

/// Eine **rohe Buchungszeile als Domänen-Wert** — das Ergebnis der Adapter-Abbildung
/// „außen → innen": das Wire-DTO `RawRecord` wird im Adapter (ACL) auf diesen Domänen-VO
/// gemappt, sodass jede Zelle bereits als `NonEmptyString` am Domänen-Rand geprüft vorliegt.
///
/// **Präsenz lebt im Typ** (ADR-0005: illegale Zustände unkonstruierbar, nicht „geprüft"):
/// ein Pflichtfeld ist **non-`Option`** — dass es da ist, garantiert die Adapter-Abbildung
/// (fehlt es dort, scheitert der ganze Batch terminal, bevor eine `RawLine` überhaupt entsteht).
/// Ein genuin optionales Datenfeld bleibt `Option`. Der Datenquellen-Port
/// [`RawLineSource`](crate::RawLineSource) liefert einen Batch davon (das Kommando ist
/// reiner Auslöser); das [`LineAggregate`](crate::LineAggregate) deutet eine `RawLine`
/// weiter zu reichen Wert-Objekten — die Domäne spricht nie ein primitives `String`.
#[derive(Debug, Clone)]
pub struct RawLine {
    /// Der Zeilen-Schlüssel, den die Quelle vergibt (opak; siehe [`RowKey`](crate::RowKey)) —
    /// Pflicht wie die Pflichtfelder: ohne ihn ließe sich ein erneuter Import nicht erkennen.
    pub row_key: NonEmptyString,
    pub ordering_account: NonEmptyString,
    pub booking_day: NonEmptyString,
    pub value_date: NonEmptyString,
    pub amount: NonEmptyString,
    pub currency: NonEmptyString,
    pub recipient_name: Option<NonEmptyString>,
    pub creditor_id: Option<NonEmptyString>,
    pub counter_account: Option<NonEmptyString>,
    pub purpose: Option<NonEmptyString>,
    pub booking_text: Option<NonEmptyString>,
}
