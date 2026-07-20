use crate::value_objects::{Amount, DayStamp, Direction};

/// Eine **Buchung des eingefrorenen Fensters**, wie der Lese-Port
/// [`AnalysisWindowSource`](crate::AnalysisWindowSource) sie liefert — genau das, was der Cashflow
/// von ihr braucht: ihren Buchungstag (der den Monats-Bucket bestimmt), ihren vorzeichenlosen
/// Betrag und ihre Richtung.
///
/// Ein **Wert**, keine Entität: dieser Slice adressiert nie eine einzelne Buchung, er reduziert über
/// sie. Ihre Identität bleibt als Fremdschlüssel im `Auswertungsfenster` von `fenstern` — hierher
/// mitzunehmen, was niemand liest, hieße Zustand ohne Leser zu führen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowedBooking {
    day: DayStamp,
    amount: Amount,
    direction: Direction,
}

impl WindowedBooking {
    /// **Hydratisiert** die Buchung aus ihren bereits geprüften Teilen — keine frische Identität,
    /// sondern Rekonstitution einer stromaufwärts schon eingefrorenen Zeile.
    #[must_use]
    pub fn hydrate(day: DayStamp, amount: Amount, direction: Direction) -> Self {
        Self {
            day,
            amount,
            direction,
        }
    }

    pub(crate) fn day(&self) -> DayStamp {
        self.day
    }

    pub(crate) fn amount(&self) -> Amount {
        self.amount
    }

    pub(crate) fn direction(&self) -> Direction {
        self.direction
    }
}
