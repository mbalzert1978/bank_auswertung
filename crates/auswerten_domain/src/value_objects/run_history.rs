use crate::value_objects::{SeedMarker, WindowedBooking};

/// Die **Historie eines Laufs**, wie der Lese-Port der fenster-übergreifenden Reichweite sie liefert
/// ([`OccurrenceSource`](crate::OccurrenceSource)): die Buchungen **aller** Auswertungsfenster des
/// Laufs, dazu das Kennzeichen der Tokenisierung, unter der ihre Token entstanden.
///
/// **Warum der Seed-Marker mitkommt.** Die Wiederkehr-Erkennung nettet Umbuchungen heraus und
/// vergleicht dafür Gegenkonto-Token gegen die Eigene-Konten-Config — derselbe Vergleich, den die
/// pro-Fenster-Reichweite führt, und mit demselben stillen Fehler, wenn beide Seiten aus verschiedenen
/// Tokenisierungen stammen: jede Umbuchung liefe als „fremd" durch und erschiene als wiederkehrender
/// Kosten-Treiber. Der Marker macht die Voraussetzung **prüfbar** statt angenommen (ADR-0010).
///
/// **Leer ist eine gültige Historie — und sie trägt dann keinen Marker.** Ein Lauf ohne Buchung in
/// der Spanne hat keine Token, also auch keine Tokenisierung. „Buchungen ohne Marker" und „Marker
/// ohne Buchungen" sind beide **unkonstruierbar** (ADR-0005).
///
/// Ein **Wert**, keine Entität: sie hat keine Identität und wird nie abgelegt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHistory {
    recorded: Option<(SeedMarker, Vec<WindowedBooking>)>,
}

impl RunHistory {
    /// Die leere Historie: in der Lauf-Spanne liegt nichts.
    #[must_use]
    pub fn empty() -> Self {
        Self { recorded: None }
    }

    /// **Hydratisiert** die Historie aus dem, was der Lese-Port für die Spanne geliefert hat. Ohne
    /// Buchungen ist sie die [`empty`](Self::empty) Historie — der Marker fällt dann weg, denn
    /// „Marker ohne Buchungen" behauptete eine Tokenisierung, von der nichts vorliegt.
    #[must_use]
    pub fn rehydrate(seed_marker: SeedMarker, bookings: Vec<WindowedBooking>) -> Self {
        match bookings.is_empty() {
            true => Self::empty(),
            false => Self {
                recorded: Some((seed_marker, bookings)),
            },
        }
    }

    /// Der Seed-Marker samt den Buchungen — `None` für die leere Historie.
    pub(crate) fn recorded(&self) -> Option<(&SeedMarker, &[WindowedBooking])> {
        self.recorded
            .as_ref()
            .map(|(marker, bookings)| (marker, bookings.as_slice()))
    }
}
