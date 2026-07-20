use std::collections::BTreeMap;

use crate::entities::AnalysisWindow;
use crate::value_objects::{Currency, Month, MonthlyCashflow, WindowId};

/// Das **Pro-Fenster-Ergebnis** — was dieser Slice aus **einem** eingefrorenen Auswertungsfenster
/// rechnet und ablegt (ADR-0013, Entscheidung E, Reichweite 1). Heute trägt es die erste periodische
/// Kennzahl: den **Cashflow je Monat innerhalb** des Fensters. Hier docken die Fach-Analyzer später
/// an (Netting, Gebühren-Klassifikation, Pareto).
///
/// Es referenziert sein Quell-Fenster **nur per ID** (ADR-0012) — die Buchungen bleiben, wo sie
/// eingefroren sind. Seine **eigene** Identität vergibt die Persistenz beim Ablegen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowEvaluation {
    window: WindowId,
    currency: Currency,
    months: Vec<MonthlyCashflow>,
}

impl WindowEvaluation {
    /// **Wertet das Fenster aus** — der einzige Konstruktor und der fachliche Kern des Handlers:
    /// jede Buchung fällt in den Monat ihres Buchungstags und wächst dort die Spalte ihrer Richtung.
    ///
    /// **Ein Fenster deckt mehrere Monate ab** — es ist die *Seitengröße* des Pagers (~400 Tage),
    /// nicht der Bucketing-*Zeitraum* (CONTEXT.md, „Fenster vs. Zeitraum"). Die `BTreeMap` fällt
    /// darum genau richtig: sie legt die Buckets **chronologisch** ab (der [`Month`] ist ein
    /// sortierbares Ordinal) und faltet gleiche Monate zusammen, ohne dass irgendwo sortiert oder
    /// gesucht werden müsste. Monate **ohne** Buchung entstehen gar nicht erst — ein leerer Bucket
    /// wäre eine Behauptung über einen Zeitraum, aus dem nichts vorliegt.
    #[must_use]
    pub(crate) fn of(window: WindowId, snapshot: &AnalysisWindow) -> Self {
        let months = snapshot
            .bookings()
            .iter()
            .fold(BTreeMap::new(), |mut buckets, booking| {
                let month = Month::of(booking.day());
                let bucket = buckets
                    .entry(month)
                    .or_insert_with(|| MonthlyCashflow::empty(month));
                *bucket = bucket.plus(booking.amount(), booking.direction());
                buckets
            });
        Self {
            window,
            currency: snapshot.currency().clone(),
            months: months.into_values().collect(),
        }
    }

    #[must_use]
    pub fn window(&self) -> &WindowId {
        &self.window
    }

    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Die Monats-Buckets in **chronologischer** Reihenfolge.
    #[must_use]
    pub fn months(&self) -> &[MonthlyCashflow] {
        &self.months
    }
}
