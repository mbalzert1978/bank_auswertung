use std::collections::BTreeMap;

use crate::abstractions::{OccurrenceSeam, SourceUnavailable};
use crate::models::WindowBookingRow;

/// Fake des fenster-übergreifenden Lese-Seams: hält die arrangierten Fenster **je ID** im Speicher
/// und spielt so die vertagte Ablage für die Specs.
///
/// `bookings_in` liefert die Buchungen **aller** Fenster — gefiltert auf die Lauf-Spanne. Dieses
/// Filtern ist der Punkt: es spielt die `WHERE`-Klausel, die io später über die Fenster-Tabelle
/// legt. Nur dadurch kann ein Spec beobachten, ob der Handler die Spanne wirklich **durchreicht** —
/// ignorierte er sie, flössen die Aggregate eines früheren Laufs still mit ein.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Beschaffungs-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeOccurrenceSource {
    /// Die Fenster **je ID** — die `BTreeMap` legt sie in stabiler, eingabe-unabhängiger
    /// Reihenfolge ab, damit der Fake nicht selbst zur Quelle von Nichtdeterminismus wird.
    windows: BTreeMap<String, Vec<WindowBookingRow>>,
    failure: Option<String>,
}

impl FakeOccurrenceSource {
    /// Arrangiert die Buchungen eines eingefrorenen Auswertungsfensters unter seiner ID.
    pub(crate) fn insert(&mut self, window_id: &str, bookings: Vec<WindowBookingRow>) {
        self.windows.insert(window_id.to_string(), bookings);
    }

    /// Lässt jede Beschaffung terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }
}

impl OccurrenceSeam for FakeOccurrenceSource {
    fn bookings_in(&self, from: i64, to: i64) -> Result<Vec<WindowBookingRow>, SourceUnavailable> {
        if let Some(message) = self.failure.clone() {
            return Err(SourceUnavailable { message });
        }
        Ok(self
            .windows
            .values()
            .flatten()
            // Die Lauf-Spanne als Scope, **beide Grenzen einschließlich** — genau das, was io als
            // `WHERE booking_day BETWEEN ? AND ?` formulieren wird.
            .filter(|booking| (from..=to).contains(&booking.booking_day))
            .cloned()
            .collect())
    }
}
