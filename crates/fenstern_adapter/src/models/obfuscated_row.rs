use fenstern_domain::{BookingId, Currency, NonEmptyString, WindowedBooking};

/// Die **obfuskierte Zeile**, wie sie der Lese-Seam aus der `obfuskiert`-Ablage liefert — genau die
/// zwei Zellen, die das Fenster verwertet: die ID (der Fremdschlüssel, den es einfriert) und die
/// Währung (auf der die eine-Währung-Invariante sitzt). Alles Übrige bleibt in der Ablage; das
/// Fenster referenziert Fremd-Entitäten **nur per ID** (ADR-0012).
///
/// Reines Zwischen-DTO zwischen dem Seam (io) und der Domäne; die Abbildung „außen → innen" lebt
/// hier (ACL). **Kein serde**: io baut die Zeile aus dem Speicher, sie wird nicht aus einer
/// Nachricht deserialisiert (die Nachricht trägt nur den Anker).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObfuscatedRow {
    pub id: String,
    pub currency: String,
}

impl ObfuscatedRow {
    /// Deutet die rohe Zeile in die Domänen-Sicht [`WindowedBooking`] (ACL, „außen → innen").
    /// **Hier erzwingt der Adapter die Präsenz** der Pflichtfelder: fehlt/ist eines leer, wird es zu
    /// `Err(MissingMandatoryField)` mit **nur** dem Feldnamen — den die
    /// [`SourceBridge`](crate::adapter::SourceBridge) auf den terminalen
    /// `WindowError::MissingMandatoryField` mappt (redigiert, statischer Feldname).
    pub(crate) fn to_booking(&self) -> Result<WindowedBooking, MissingMandatoryField> {
        Ok(WindowedBooking::hydrate(
            BookingId::new(mandatory(&self.id, "id")?.as_str()),
            Currency::from(mandatory(&self.currency, "currency")?),
        ))
    }
}

/// Ein Pflichtfeld fehlt (leer/nicht vorhanden) in der obfuskierten Zeile — **terminaler** Ausgang
/// der „außen → innen"-Abbildung. **Redaktions-Invariante:** trägt **nur** den statischen
/// Feldnamen, nie den Rohwert — auch `Debug` spiegelt so keinen Identifikator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingMandatoryField {
    field: &'static str,
}

impl MissingMandatoryField {
    /// Der (statische, unsensible) Name des fehlenden Pflichtfelds — für den redigierten terminalen
    /// Fehlergrund.
    pub(crate) fn field(&self) -> &'static str {
        self.field
    }
}

/// Erzwingt ein Pflichtfeld: reicht die Zelle als `NonEmptyString` an die Domänengrenze, oder —
/// fehlt/leer sie — den redigierten [`MissingMandatoryField`] mit nur dem Feldnamen.
fn mandatory(raw: &str, field: &'static str) -> Result<NonEmptyString, MissingMandatoryField> {
    NonEmptyString::new(raw).ok_or(MissingMandatoryField { field })
}
