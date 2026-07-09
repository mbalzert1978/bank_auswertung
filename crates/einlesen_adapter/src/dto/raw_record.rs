use std::fmt;

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
    /// Bildet den Wire-DTO auf den Domänen-VO [`RawLine`] ab (ACL, „außen → innen"). **Hier
    /// erzwingt der Adapter die Präsenz** der Pflichtfelder: ein fehlendes/leeres Pflichtfeld
    /// wird zu `Err(MissingMandatoryField)` (die Domäne *fordert* Präsenz im Typ, prüft sie
    /// aber nicht — das ist ACL-Mechanik). Optionale Zellen werden zu `Option<NonEmptyString>`.
    pub(crate) fn to_raw_line(&self) -> Result<RawLine, MissingMandatoryField> {
        Ok(RawLine {
            ordering_account: mandatory(&self.ordering_account, "ordering_account")?,
            booking_day: mandatory(&self.booking_day, "booking_day")?,
            value_date: mandatory(&self.value_date, "value_date")?,
            amount: mandatory(&self.amount, "amount")?,
            currency: mandatory(&self.currency, "currency")?,
            recipient_name: as_nonempty(&self.recipient_name),
            creditor_id: as_nonempty(&self.creditor_id),
            counter_account: as_nonempty(&self.counter_account),
            purpose: as_nonempty(&self.purpose),
            booking_text: as_nonempty(&self.booking_text),
        })
    }
}

/// Ein Pflichtfeld fehlt (leer/nicht vorhanden) in der Rohzeile — **terminaler**
/// Adapter-Fehler der „außen → innen"-Abbildung; die Consume-Bridge mappt ihn auf ein
/// Dead-Letter. **Datenschutz (Redaktions-Invariante):** trägt **nur** den statischen
/// Feldnamen, nie den Rohwert — auch `Debug` spiegelt so keinen Identifikator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingMandatoryField {
    field: &'static str,
}

impl fmt::Display for MissingMandatoryField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mandatory field missing: {}", self.field)
    }
}

impl std::error::Error for MissingMandatoryField {}

/// Erzwingt ein Pflichtfeld: reicht die Zelle als `NonEmptyString` an die Domänengrenze,
/// oder — fehlt/leer sie — den redigierten [`MissingMandatoryField`] mit nur dem Feldnamen.
fn mandatory(
    raw: &Option<String>,
    field: &'static str,
) -> Result<NonEmptyString, MissingMandatoryField> {
    as_nonempty(raw).ok_or(MissingMandatoryField { field })
}

/// Reicht eine optionale rohe Zelle als `NonEmptyString` an die Domänengrenze (leer/`None` →
/// `None`); der `NonEmptyString`-Kern trimmt und weist Leeres genau hier ab.
fn as_nonempty(raw: &Option<String>) -> Option<NonEmptyString> {
    raw.as_deref().and_then(NonEmptyString::new)
}
