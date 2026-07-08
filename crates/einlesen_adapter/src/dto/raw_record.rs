use einlesen_domain::{NonEmptyString, RawLine};
use serde::{Deserialize, Serialize};

/// Der rohe Datensatz **einer** Zeile: die zehn verwerteten CSV-CAMT-v8-Felder als rohe Zellen
/// (leer/fehlend = `None`). Reines **Wire-/Eingangs-DTO** (öffentliche Felder, serde) — die
/// öffentliche Fläche, die die TestApi arrangiert und io in die Outbox schreibt. Die Abbildung
/// **„außen → innen"** (Wire-DTO → Domänen-VO [`RawLine`]) lebt hier im Adapter (ACL):
/// [`to_raw_line`](RawRecord::to_raw_line) reicht jede Zelle als `NonEmptyString` an den
/// Domänen-Rand — **wie** eine CSV-Spalte einem Feld zugeordnet wird, kapselt der Adapter, die
/// Domäne bekommt nur geprüfte Werte. Test-Fixtures leben **in den Specs**, nicht hier.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawRecord {
    pub ordering_account: Option<String>,
    pub booking_day: Option<String>,
    pub value_date: Option<String>,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub recipient_name: Option<String>,
    pub creditor_id: Option<String>,
    pub counter_account: Option<String>,
    pub purpose: Option<String>,
    pub booking_text: Option<String>,
}

impl RawRecord {
    /// Bildet den Wire-DTO auf den Domänen-VO [`RawLine`] ab (ACL, „außen → innen"):
    /// jede rohe Zelle wird zu `Option<NonEmptyString>` am Domänen-Rand geprüft.
    pub(crate) fn to_raw_line(&self) -> RawLine {
        RawLine {
            ordering_account: as_nonempty(&self.ordering_account),
            booking_day: as_nonempty(&self.booking_day),
            value_date: as_nonempty(&self.value_date),
            amount: as_nonempty(&self.amount),
            currency: as_nonempty(&self.currency),
            recipient_name: as_nonempty(&self.recipient_name),
            creditor_id: as_nonempty(&self.creditor_id),
            counter_account: as_nonempty(&self.counter_account),
            purpose: as_nonempty(&self.purpose),
            booking_text: as_nonempty(&self.booking_text),
        }
    }
}

/// Reicht eine rohe Zelle als `NonEmptyString` an die Domänengrenze (leer/`None` → `None`); der
/// `NonEmptyString`-Kern trimmt und weist Leeres genau hier ab.
fn as_nonempty(raw: &Option<String>) -> Option<NonEmptyString> {
    raw.as_deref().and_then(NonEmptyString::new)
}
