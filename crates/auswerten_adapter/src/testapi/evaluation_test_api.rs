use application_kernel::{CommandDecoder, Handler};
use auswerten_domain::EvaluationError;

use super::fakes::{FakeAnalysisWindowSource, FakeEvaluationStore};
use crate::handler::WindowEvaluationHandler;
use crate::models::{AnalysisWindowRow, EvaluateWindowCommandDecoder, WindowEvaluationRow};

/// Fluent-TestApi des auswerten-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md),
/// [ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)): die
/// **Eingangsfläche spricht nur DTOs** — mit [`with_window`](Self::with_window) arrangiert man ein
/// eingefrorenes Auswertungsfenster unter seiner ID, [`run`](Self::run) fährt dann für **eine ID**
/// die **echte** Kette über den `application_kernel`-Vertrag — [`EvaluateWindowCommandDecoder`]
/// (Envelope-Strings → Kommando, Shape Y) → [`WindowEvaluationHandler`](crate::WindowEvaluationHandler)
/// (Quell-/Repository-Bridge + domänen-interner Handler) gegen öffentliche Fakes. **Keine
/// Outbox-Mechanik:** der Slice ist eine reine Pipeline; `OutboxConsumer`/`UnitOfWork` prüft der
/// io-Integrationstest, nicht dieser Spec.
///
/// `run` bekommt **nur die ID** — genau die Lage des Handlers in Produktion: die Nachricht ist ein
/// Zeiger, das Aggregat holt er selbst. Zurück kommt der **nackte Domänen-[`EvaluationError`]**: ein
/// ins Leere zeigender Zeiger, ein fehlendes Pflichtfeld oder ein Fehlschlag beim Lesen/Ablegen
/// lassen ihn **terminal** scheitern.
///
/// Die zwei `with_failing_*`-Schalter spielen den terminalen Fehlschlag der jeweiligen Richtung — so
/// trifft ein Spec **beide** fehlbaren I/O-Schritte black-box (ADR-0012).
pub struct EvaluationTestApi {
    source: FakeAnalysisWindowSource,
    store: FakeEvaluationStore,
}

impl EvaluationTestApi {
    /// Neue TestApi mit leerer Fenster-Ablage und leerer Ergebnis-Tabelle.
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: FakeAnalysisWindowSource::default(),
            store: FakeEvaluationStore::default(),
        }
    }

    /// Arrangiert ein eingefrorenes Auswertungsfenster unter seiner ID in der (Fake-)Fenster-Ablage
    /// — das Aggregat, das der Handler beim Lauf über den Zeiger liest.
    #[must_use]
    pub fn with_window(mut self, window_id: &str, window: AnalysisWindowRow) -> Self {
        self.source.insert(window_id, window);
        self
    }

    /// Lässt das **Lesen** terminal scheitern (io-Diagnose durchgereicht).
    #[must_use]
    pub fn with_failing_source(mut self, message: &str) -> Self {
        self.source.fail_with(message);
        self
    }

    /// Lässt das **Ablegen** terminal scheitern (io-Diagnose durchgereicht).
    #[must_use]
    pub fn with_failing_store(mut self, message: &str) -> Self {
        self.store.fail_with(message);
        self
    }

    /// Fährt die echte Kette (Decoder → Handler) für **eine** Fenster-ID und gibt das
    /// Handler-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` = terminaler [`EvaluationError`]).
    ///
    /// # Panics
    /// Wenn die ID nicht decodiert — eine leere Envelope ist eine io-Sorge, nicht Handler-Eingabe;
    /// die Specs reichen stets einen echten Zeiger herein.
    pub fn run(&self, window_id: &str) -> Result<(), EvaluationError> {
        let command =
            EvaluateWindowCommandDecoder::decode(window_id, "").expect("Fenster-ID als Zeiger");
        WindowEvaluationHandler::new(&self.source, &self.store).handle(command)
    }

    // --- Validierung des Ergebnisses ---------------------------------------------------------

    /// Prüft, dass **genau das** erwartete Pro-Fenster-Ergebnis in der Ergebnis-Tabelle liegt (ganze
    /// Zeile, ein Vergleich) — und damit zugleich, dass es **genau eines** ist.
    pub fn assert_stored(&self, expected: WindowEvaluationRow) {
        assert_eq!(self.store.evaluations(), vec![expected]);
    }

    /// Prüft, dass **nichts** abgelegt wurde (terminaler Fehler vor der Ablage).
    pub fn assert_nothing_stored(&self) {
        assert!(
            self.store.evaluations().is_empty(),
            "es wurde ein Ergebnis abgelegt"
        );
    }
}

impl Default for EvaluationTestApi {
    fn default() -> Self {
        Self::new()
    }
}
