use auswerten_domain::{Direction, Occurrence};

use crate::models::BookingDirection;

/// Ein **Vorkommen** einer Buchungsart auf der Zeitachse des Laufs, als Ausgangs-DTO — das, was die
/// fenster-übergreifende Reichweite zurückgibt (ADR-0013, Entscheidung E, Reichweite 2).
///
/// Es trägt **genau vier** Angaben, jede mit einem Abnehmer in der Wiederkehr-Erkennung
/// ([0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)): der Fingerabdruck gruppiert zu
/// einer Art, der Buchungstag trägt den **Rhythmus**, Betrag und Richtung den **Betragsvergleich**.
///
/// **Nicht persistiert:** anders als die [`WindowEvaluationRow`](crate::WindowEvaluationRow) geht
/// diese Zeile in **keine** Ablage — es gibt kein Vorkommens-/Fold-Log. Sie ist die Antwort der
/// Query, nicht eine zweite gespeicherte Sicht.
///
/// **Kein serde**: sie wird zurückgegeben, nicht serialisiert. Sollte sie je an eine **Egress**-DTO
/// andocken (Kategorisierer-Anfrage, Report), gilt dort die `assert_not_impl_any!`-Linie der
/// Datenschutz-Grenze — der Fingerabdruck selbst trägt nur öffentliche Anker oder seed-basierte
/// Pseudonyme, nie einen Rohwert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceRow {
    /// Der Fingerabdruck der Buchungs*art* — der Schlüssel, unter dem die Vorkommen über die Fenster
    /// hinweg zusammenfinden.
    pub fingerprint: String,
    /// Der Buchungstag als sortierbarer Tag-Stamp (ADR-0013, Entscheidung C).
    pub booking_day: i64,
    pub amount_cent: u64,
    pub direction: BookingDirection,
}

impl From<&Occurrence> for OccurrenceRow {
    /// Setzt das Domänen-Vorkommen auf die rohe Zeile um (ACL, „innen → außen").
    fn from(occurrence: &Occurrence) -> Self {
        Self {
            fingerprint: occurrence.fingerprint().as_str().to_string(),
            booking_day: occurrence.day().value(),
            amount_cent: occurrence.amount().cent(),
            direction: match occurrence.direction() {
                Direction::Incoming => BookingDirection::Incoming,
                Direction::Outgoing => BookingDirection::Outgoing,
            },
        }
    }
}
