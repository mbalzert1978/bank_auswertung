use application_kernel::{CommandDecoder, Handler};
use auswerten_domain::EvaluationError;

use super::fakes::FakeOccurrenceSource;
use crate::history_handler::HistoryEvaluationHandler;
use crate::models::{EvaluateHistoryCommandDecoder, OccurrenceRow, WindowBookingRow};

/// Fluent-TestApi der **fenster-übergreifenden** Reichweite (ADR-0013, Entscheidung E, Reichweite
/// 2): die **Eingangsfläche spricht nur DTOs** — mit
/// [`with_window_bookings`](Self::with_window_bookings) arrangiert man die Buchungen eines
/// eingefrorenen Fensters unter seiner ID, [`run_history`](Self::run_history) fährt dann die
/// **echte** Kette über den `application_kernel`-Vertrag — [`EvaluateHistoryCommandDecoder`]
/// (Envelope-Strings → Kommando, Shape Y) → [`HistoryEvaluationHandler`](crate::HistoryEvaluationHandler)
/// (Lese-Bridge + domänen-interner Handler) gegen einen öffentlichen Fake.
///
/// `run_history` bekommt **die Lauf-Spanne**, genau wie der Handler in Produktion: sie reist im
/// `partition_key` des `HistorieVollständig`-Signals, das `fenstern` beim Versiegen der Historie
/// publiziert. Zurück kommt die **Sicht selbst** — die [`OccurrenceRow`]s der ganzen Zeitachse — oder
/// der **nackte Domänen-[`EvaluationError`]**.
///
/// **Es gibt bewusst kein `assert_stored`.** Diese Reichweite legt nichts ab: kein Vorkommens-/
/// Fold-Log, keine zweite persistierte Sicht. Was sie tut, steht vollständig in ihrem Rückgabewert —
/// eine TestApi mit einem Ablage-Assert würde eine Ablage suggerieren, die es nicht gibt.
pub struct HistoryTestApi {
    source: FakeOccurrenceSource,
}

impl HistoryTestApi {
    /// Neue TestApi mit leerer Fenster-Ablage.
    ///
    /// **Keine Eigene-Konten-Config, bewusst:** dieser Handler ordnet keine Gegenkonten ein und
    /// braucht darum keinen Seed-Marker-Abgleich — er sammelt Vorkommen ein. Einzweckig wie der
    /// erste Handler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: FakeOccurrenceSource::default(),
        }
    }

    /// Arrangiert die Buchungen eines eingefrorenen Auswertungsfensters unter seiner ID. Mehrfach
    /// aufgerufen entsteht die Lage, die diese Reichweite ausmacht: **mehrere** Fenster eines Laufs,
    /// über die hinweg gelesen wird.
    #[must_use]
    pub fn with_window_bookings(mut self, window_id: &str, bookings: &[WindowBookingRow]) -> Self {
        self.source.insert(window_id, bookings.to_vec());
        self
    }

    /// Lässt das **Lesen** terminal scheitern (io-Diagnose durchgereicht) — der eine fehlbare
    /// I/O-Schritt dieser Reichweite (ADR-0012).
    #[must_use]
    pub fn with_failing_source(mut self, message: &str) -> Self {
        self.source.fail_with(message);
        self
    }

    /// Fährt die echte Kette (Decoder → Handler) für eine **Lauf-Spanne** und gibt die Vorkommen
    /// über die ganze Zeitachse zurück (`Err` = terminaler [`EvaluationError`]).
    ///
    /// Die Spanne wird dabei **so kodiert, wie `fenstern` sie publiziert** (`"{from}:{to}"`) und
    /// durch den echten Decoder geschickt — der Spec prüft damit auch die Envelope-Form mit, nicht
    /// nur den Handler.
    ///
    /// # Panics
    /// Wenn die Spanne nicht decodiert — eine unlesbare Envelope ist eine io-Sorge, nicht
    /// Handler-Eingabe; die Specs reichen stets eine echte Spanne herein.
    pub fn run_history(&self, from: i64, to: i64) -> Result<Vec<OccurrenceRow>, EvaluationError> {
        let command = EvaluateHistoryCommandDecoder::decode(&format!("{from}:{to}"), "")
            .expect("Lauf-Spanne als Envelope");
        HistoryEvaluationHandler::new(&self.source).handle(command)
    }
}

impl Default for HistoryTestApi {
    fn default() -> Self {
        Self::new()
    }
}
