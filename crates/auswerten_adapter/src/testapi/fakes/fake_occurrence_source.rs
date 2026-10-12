use std::collections::BTreeMap;

use crate::abstractions::{OccurrenceSeam, SourceUnavailable};
use crate::models::{AnalysisWindowRow, RunHistoryRow};

/// Fake des fenster-übergreifenden Lese-Seams: hält die arrangierten Fenster **je ID** im Speicher
/// und spielt so die vertagte Ablage für die Specs.
///
/// `history_in` liefert die Buchungen **aller** Fenster — gefiltert auf die Lauf-Spanne. Dieses
/// Filtern ist der Punkt: es spielt die `WHERE`-Klausel, die io über die Fenster-Tabelle legt. Nur
/// dadurch kann ein Spec beobachten, ob der Handler die Spanne wirklich **durchreicht** — ignorierte
/// er sie, flössen die Aggregate eines früheren Laufs still mit ein.
///
/// Den **Seed-Marker** der Historie nimmt er von den Fenstern, die zur Spanne beitragen — und spielt
/// dabei io's Invariante mit: tragen sie **verschiedene**, scheitert die Beschaffung, statt still
/// einen zu wählen.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Beschaffungs-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeOccurrenceSource {
    /// Die Fenster **je ID** — die `BTreeMap` legt sie in stabiler, eingabe-unabhängiger
    /// Reihenfolge ab, damit der Fake nicht selbst zur Quelle von Nichtdeterminismus wird.
    windows: BTreeMap<String, AnalysisWindowRow>,
    failure: Option<String>,
}

impl FakeOccurrenceSource {
    /// Arrangiert ein eingefrorenes Auswertungsfenster unter seiner ID.
    pub(crate) fn insert(&mut self, window_id: &str, window: AnalysisWindowRow) {
        self.windows.insert(window_id.to_string(), window);
    }

    /// Lässt jede Beschaffung terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }
}

impl OccurrenceSeam for FakeOccurrenceSource {
    fn history_in(&self, from: i64, to: i64) -> Result<RunHistoryRow, SourceUnavailable> {
        if let Some(message) = self.failure.clone() {
            return Err(SourceUnavailable { message });
        }
        // Die Lauf-Spanne als Scope, **beide Grenzen einschließlich** — genau das, was io als
        // `WHERE booking_day BETWEEN ? AND ?` formuliert.
        let contributing = self
            .windows
            .values()
            .map(|window| {
                (
                    window.seed_marker.as_str(),
                    window
                        .bookings
                        .iter()
                        .filter(|booking| (from..=to).contains(&booking.booking_day))
                        .cloned()
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, bookings)| !bookings.is_empty())
            .collect::<Vec<_>>();
        let mut markers = contributing.iter().map(|(marker, _)| *marker);
        let seed_marker = markers.next().unwrap_or_default();
        if markers.any(|marker| marker != seed_marker) {
            return Err(SourceUnavailable {
                message: "Lauf trägt Buchungen aus unterschiedlichen Tokenisierungen".to_string(),
            });
        }
        Ok(RunHistoryRow {
            seed_marker: seed_marker.to_string(),
            bookings: contributing
                .into_iter()
                .flat_map(|(_, bookings)| bookings)
                .collect(),
        })
    }
}
