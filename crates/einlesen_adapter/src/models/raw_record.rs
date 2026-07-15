use einlesen_domain::{NonEmptyString, RawLine};

/// Der rohe Datensatz **einer** Zeile: die zehn verwerteten CSV-CAMT-v8-Felder als rohe Zellen
/// (leer/fehlend = `None`). Reines **Seam-/Eingangs-DTO** (öffentliche Felder, **kein serde** —
/// seit [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
/// reist es nicht mehr über die Outbox, sondern in-process über den
/// [`SourceSeam`](crate::abstractions::SourceSeam)) — die öffentliche Fläche, die die TestApi
/// in der Quelle arrangiert und io beim Erfüllen des Seams liefert. Die Abbildung
/// **„außen → innen"** (Seam-DTO → Domänen-VO [`RawLine`]) lebt hier im Adapter (ACL): der
/// [`TryFrom<&RawRecord>`](RawLine)-Impl reicht jede Zelle als `NonEmptyString` an den
/// Domänen-Rand — **wie** eine CSV-Spalte einem Feld zugeordnet wird, kapselt der Adapter, die
/// Domäne bekommt nur geprüfte Werte. Test-Fixtures leben **in den Specs**, nicht hier.
#[derive(Debug, Clone, Default)]
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

impl TryFrom<&RawRecord> for RawLine {
    type Error = MissingMandatoryField;

    /// Bildet das Seam-DTO auf den Domänen-VO [`RawLine`] ab (ACL, „außen → innen"). **Hier
    /// erzwingt der Adapter die Präsenz** der Pflichtfelder: ein fehlendes/leeres Pflichtfeld
    /// wird zu `Err(MissingMandatoryField)` (die Domäne *fordert* Präsenz im Typ, prüft sie
    /// aber nicht — das ist ACL-Mechanik). Optionale Zellen werden zu `Option<NonEmptyString>`.
    fn try_from(record: &RawRecord) -> Result<Self, Self::Error> {
        Ok(RawLine {
            ordering_account: mandatory(&record.ordering_account, "ordering_account")?,
            booking_day: mandatory(&record.booking_day, "booking_day")?,
            value_date: mandatory(&record.value_date, "value_date")?,
            amount: mandatory(&record.amount, "amount")?,
            currency: mandatory(&record.currency, "currency")?,
            recipient_name: as_nonempty(&record.recipient_name),
            creditor_id: as_nonempty(&record.creditor_id),
            counter_account: as_nonempty(&record.counter_account),
            purpose: as_nonempty(&record.purpose),
            booking_text: as_nonempty(&record.booking_text),
        })
    }
}

/// Ein Pflichtfeld fehlt (leer/nicht vorhanden) in der Rohzeile — **terminaler**
/// Adapter-Fehler der „außen → innen"-Abbildung; die
/// [`SourceBridge`](crate::adapter::SourceBridge) mappt ihn auf den redigierten
/// Domänen-Fehler `SourceError::MissingMandatoryField` (dessen Domänen-`Display` allein den
/// Meldungssatz besitzt — hier bewusst **kein** `Display`-Duplikat). **Datenschutz
/// (Redaktions-Invariante):** trägt **nur** den statischen Feldnamen, nie den Rohwert —
/// auch `Debug` spiegelt so keinen Identifikator.
///
/// **Sichtbarkeit:** `pub`, weil er als [`TryFrom<&RawRecord>`](RawLine)-`Error` in einer
/// öffentlichen Trait-Impl über das ebenfalls öffentliche [`RawRecord`] auftaucht (sonst
/// E0446: crate-privater Typ in öffentlicher Schnittstelle). Er bleibt aber **opak** — das
/// Feld ist privat, der [`field`](Self::field)-Zugriff `pub(crate)`; nach außen ist er nur
/// ein benennbarer, redigierter Fehlertyp ohne lesbaren Inhalt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingMandatoryField {
    field: &'static str,
}

impl MissingMandatoryField {
    /// Der (statische, unsensible) Name des fehlenden Pflichtfelds — für den redigierten
    /// Domänen-Fehler.
    pub(crate) fn field(&self) -> &'static str {
        self.field
    }
}

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
