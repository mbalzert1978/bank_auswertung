use auswerten_domain::{
    Amount, AnalysisWindow, Currency, DayStamp, Direction, NonEmptyString, WindowedBooking,
};

/// Das **eingefrorene Auswertungsfenster**, wie der Lese-Seam es zu einer ID liefert: die eine
/// Währung des Fensters und die Buchungen, auf die es per Fremdschlüssel zeigt (io setzt es aus der
/// Fenster-Zeile und ihrem Join auf die `obfuskiert`-Ablage zusammen).
///
/// Reines Zwischen-DTO zwischen dem Seam (io) und der Domäne; die Abbildung „außen → innen" lebt
/// hier (ACL). **Kein serde**: io baut die Zeile aus dem Speicher, sie wird nicht aus einer
/// Nachricht deserialisiert (die Nachricht trägt nur die ID).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnalysisWindowRow {
    pub currency: String,
    pub bookings: Vec<WindowBookingRow>,
}

/// Eine **Buchung des Fensters** als rohe Zeile — genau die drei Zellen, die der Cashflow verwertet:
/// der Buchungstag (er bestimmt den Monats-Bucket), der **vorzeichenlose** Betrag in Cent und die
/// Richtung, in der das Vorzeichen lebt (ADR-0005).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowBookingRow {
    /// Der Buchungstag als sortierbarer Tag-Stamp (ADR-0013, Entscheidung C).
    pub booking_day: i64,
    pub amount_cent: u64,
    pub direction: BookingDirection,
}

/// Die Richtung als **adapter-eigener DTO-Typ**. Bewusst **re-deklariert** statt aus der Domäne
/// re-exportiert: kein Domänen-Internum überquert die Grenze (dieselbe Linie, die `einlesen` bei
/// `Direction`/`Currency` zieht).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BookingDirection {
    Incoming,
    #[default]
    Outgoing,
}

impl AnalysisWindowRow {
    /// Deutet die rohe Zeile in das Domänen-Aggregat [`AnalysisWindow`] (ACL, „außen → innen").
    /// **Hier erzwingt der Adapter die Präsenz** der Pflichtfelder: fehlt/ist die Währung leer, wird
    /// das zu `Err(MissingMandatoryField)` mit **nur** dem Feldnamen — den die
    /// [`SourceBridge`](crate::adapter::SourceBridge) auf den terminalen
    /// `EvaluationError::MissingMandatoryField` mappt (redigiert, statischer Feldname).
    ///
    /// Buchungstag, Betrag und Richtung brauchen **keine** Präsenz-Prüfung: sie sind schon im Typ
    /// vollständig (ADR-0005 — illegale Zustände unkonstruierbar, nicht „geprüft").
    pub(crate) fn to_window(&self) -> Result<AnalysisWindow, MissingMandatoryField> {
        let currency = Currency::from(mandatory(&self.currency, "currency")?);
        Ok(AnalysisWindow::rehydrate(
            currency,
            self.bookings
                .iter()
                .copied()
                .map(WindowBookingRow::to_booking)
                .collect(),
        ))
    }
}

impl WindowBookingRow {
    /// Deutet die rohe Buchungs-Zeile in die Domänen-Sicht — **unfehlbar**, weil jede ihrer Zellen
    /// schon im Typ vollständig ist.
    fn to_booking(self) -> WindowedBooking {
        WindowedBooking::hydrate(
            DayStamp::new(self.booking_day),
            Amount::from_cent(self.amount_cent),
            match self.direction {
                BookingDirection::Incoming => Direction::Incoming,
                BookingDirection::Outgoing => Direction::Outgoing,
            },
        )
    }
}

/// Ein Pflichtfeld fehlt (leer/nicht vorhanden) im gelesenen Fenster — **terminaler** Ausgang der
/// „außen → innen"-Abbildung. **Redaktions-Invariante:** trägt **nur** den statischen Feldnamen, nie
/// den Rohwert — auch `Debug` spiegelt so keinen Inhalt.
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
