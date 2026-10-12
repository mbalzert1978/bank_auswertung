use std::collections::BTreeSet;

use crate::error::EvaluationError;
use crate::value_objects::{Currency, SeedMarker, WindowedBooking};

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
/// **Eine Währung für den ganzen Lauf.** Jedes Fenster hat sie an seiner Konstruktion erzwungen —
/// aber nur für sich. Diese Reichweite summiert über **alle** Fenster des Laufs (Cashflow je Konto,
/// Ranking, Jahresbeträge); trügen zwei Fenster verschiedene Währungen, addierte sie Euro zu Dollar
/// und das Ergebnis läse sich trotzdem richtig. Darum entsteht eine Historie nur über **genau eine**
/// Währung ([`rehydrate`](Self::rehydrate) weist mehrere terminal ab) — gemischt ist unkonstruierbar.
///
/// Ein **Wert**, keine Entität: sie hat keine Identität und wird nie abgelegt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHistory {
    recorded: Option<(SeedMarker, Currency, Vec<WindowedBooking>)>,
}

impl RunHistory {
    /// Die leere Historie: in der Lauf-Spanne liegt nichts.
    #[must_use]
    pub fn empty() -> Self {
        Self { recorded: None }
    }

    /// **Hydratisiert** die Historie aus dem, was der Lese-Port für die Spanne geliefert hat: den
    /// Seed-Marker, die Währungen der beteiligten Fenster und die Buchungen. Ohne Buchungen ist sie die
    /// [`empty`](Self::empty) Historie — Marker und Währung fallen dann weg, denn sie behaupteten
    /// etwas, von dem nichts vorliegt.
    ///
    /// **Mit** Buchungen muss es **genau eine** Währung sein: mehrere sind
    /// [`EvaluationError::MixedCurrencies`] (terminal, ohne die Werte zu nennen), keine ist ein
    /// fehlendes Pflichtfeld.
    pub fn rehydrate(
        seed_marker: SeedMarker,
        currencies: Vec<Currency>,
        bookings: Vec<WindowedBooking>,
    ) -> Result<Self, EvaluationError> {
        if bookings.is_empty() {
            return Ok(Self::empty());
        }
        let mut currencies = currencies.into_iter().collect::<BTreeSet<_>>().into_iter();
        match (currencies.next(), currencies.next()) {
            (Some(currency), None) => Ok(Self {
                recorded: Some((seed_marker, currency, bookings)),
            }),
            (Some(_), Some(_)) => Err(EvaluationError::MixedCurrencies),
            (None, _) => Err(EvaluationError::MissingMandatoryField { field: "currency" }),
        }
    }

    /// Seed-Marker, Währung und Buchungen — `None` für die leere Historie.
    pub(crate) fn recorded(&self) -> Option<(&SeedMarker, &Currency, &[WindowedBooking])> {
        self.recorded
            .as_ref()
            .map(|(marker, currency, bookings)| (marker, currency, bookings.as_slice()))
    }
}
