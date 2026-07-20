use crate::value_objects::{Currency, WindowedBooking};

/// Das **eingefrorene Auswertungsfenster**, wie dieser Slice es **per ID** zurückliest — die
/// Einheit, über die die periodische Auswertung **innerhalb** rechnet (ADR-0013, Entscheidung E,
/// Reichweite 1). Es trägt die **eine** Währung des Fensters und seine Buchungen.
///
/// **Rehydratisierung, keine Neukonstruktion:** die Invarianten des Fensters (nicht leer, eine
/// Währung) sind schon **an der Konstruktion** in `fenstern` entstanden (ADR-0013, Entscheidung A).
/// Dieser Slice liest einen Beweis, der bereits geführt ist — ihn hier erneut zu führen wäre eine
/// zweite, divergierbare Kopie derselben Regel. Was er stattdessen **nicht** kann, ist ein Fenster
/// erfinden: das Aggregat entsteht nur aus dem, was die Ablage hergibt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisWindow {
    currency: Currency,
    bookings: Vec<WindowedBooking>,
}

impl AnalysisWindow {
    /// **Hydratisiert** das Fenster aus dem, was der Lese-Port zur ID geliefert hat.
    #[must_use]
    pub fn rehydrate(currency: Currency, bookings: Vec<WindowedBooking>) -> Self {
        Self { currency, bookings }
    }

    pub(crate) fn currency(&self) -> &Currency {
        &self.currency
    }

    pub(crate) fn bookings(&self) -> &[WindowedBooking] {
        &self.bookings
    }
}
