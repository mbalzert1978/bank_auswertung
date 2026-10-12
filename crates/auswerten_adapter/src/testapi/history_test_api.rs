use std::num::NonZeroU64;

use application_kernel::{CommandDecoder, Handler};
use auswerten_domain::EvaluationError;

use super::fakes::FakeOccurrenceSource;
use crate::history_handler::HistoryEvaluationHandler;
use crate::models::{
    AnalysisWindowRow, EvaluateHistoryCommandDecoder, HistoryEvaluationRow, OwnAccountsConfig,
    SavingsThresholdConfig,
};

/// Fluent-TestApi der **fenster-übergreifenden** Reichweite (ADR-0013, Entscheidung E, Reichweite
/// 2): die **Eingangsfläche spricht nur DTOs** — mit [`with_window`](Self::with_window) arrangiert
/// man ein eingefrorenes Fenster unter seiner ID, mit [`with_own_accounts`](Self::with_own_accounts)
/// die Eigene-Konten-Config, und [`run_history`](Self::run_history) fährt dann die **echte** Kette
/// über den `application_kernel`-Vertrag — [`EvaluateHistoryCommandDecoder`] (Envelope-Strings →
/// Kommando, Shape Y) → [`HistoryEvaluationHandler`](crate::HistoryEvaluationHandler) (Lese-Bridge +
/// domänen-interner Handler) gegen einen öffentlichen Fake. **Kein Datei-, kein Netzzugriff:** alles
/// lebt im Speicher.
///
/// `run_history` bekommt **die Lauf-Spanne**, genau wie der Handler in Produktion: sie reist im
/// `partition_key` des `HistorieVollständig`-Signals, das `fenstern` beim Versiegen der Historie
/// publiziert. Zurück kommt die **Sicht selbst** — die [`HistoryEvaluationRow`] mit den Vorkommen der
/// ganzen Zeitachse und den wiederkehrenden Abflüssen — oder der **nackte
/// Domänen-[`EvaluationError`]**.
///
/// **Es gibt bewusst kein `assert_stored`.** Diese Reichweite legt nichts ab: kein Vorkommens-/
/// Fold-Log, keine zweite persistierte Sicht. Was sie tut, steht vollständig in ihrem Rückgabewert —
/// eine TestApi mit einem Ablage-Assert würde eine Ablage suggerieren, die es nicht gibt.
pub struct HistoryTestApi {
    source: FakeOccurrenceSource,
    own_accounts: OwnAccountsConfig,
    savings_threshold: SavingsThresholdConfig,
}

impl HistoryTestApi {
    /// Der Lauf, dem [`with_window`](Self::with_window) ein Fenster zuordnet und dessen Abschluss
    /// [`run_history`](Self::run_history) fährt.
    pub const DEFAULT_RUN: &str = "lauf-1";

    /// Neue TestApi mit leerer Fenster-Ablage und **noch nicht arrangierter** Eigene-Konten-Config.
    ///
    /// **[`with_own_accounts`](Self::with_own_accounts) ist Pflicht, nicht Kür** — dieselbe Linie wie
    /// bei der [`EvaluationTestApi`](crate::EvaluationTestApi): die Nullwert-Config trägt keinen
    /// Seed-Marker, und ein [`run_history`](Self::run_history) darauf scheitert terminal, **bevor**
    /// irgendetwas gelesen wird.
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: FakeOccurrenceSource::default(),
            own_accounts: OwnAccountsConfig::default(),
            savings_threshold: SavingsThresholdConfig {
                annual_minor_units: Self::UNREACHABLE_THRESHOLD,
            },
        }
    }

    /// Die Spar-Schwelle, solange keine arrangiert ist: **unerreichbar hoch**, sodass kein Treiber
    /// Kandidat wird. Wer Kandidaten prüfen will, setzt die Schwelle ausdrücklich
    /// ([`with_savings_threshold`](Self::with_savings_threshold)) — ein stiller Vorgabewert, der
    /// Kandidaten erzeugte, machte jede Erwartung davon abhängig, ihn zu kennen.
    pub const UNREACHABLE_THRESHOLD: NonZeroU64 = NonZeroU64::MAX;

    /// Reicht die **Spar-Schwelle** als Wert herein: ab diesem Jahresbetrag (Minor Units) wird ein
    /// Kostentreiber Spar-Kandidat.
    #[must_use]
    pub fn with_savings_threshold(mut self, annual_minor_units: NonZeroU64) -> Self {
        self.savings_threshold = SavingsThresholdConfig { annual_minor_units };
        self
    }

    /// Reicht die **Eigene-Konten-Config als Wert** herein. Ihr Seed-Marker muss der der arrangierten
    /// Fenster sein — sonst endet der Lauf terminal mit `SeedMismatch`.
    #[must_use]
    pub fn with_own_accounts(mut self, own_accounts: OwnAccountsConfig) -> Self {
        self.own_accounts = own_accounts;
        self
    }

    /// Arrangiert ein eingefrorenes Auswertungsfenster unter seiner ID. Mehrfach aufgerufen entsteht
    /// die Lage, die diese Reichweite ausmacht: **mehrere** Fenster eines Laufs, über die hinweg
    /// gelesen wird.
    #[must_use]
    pub fn with_window(mut self, window_id: &str, window: AnalysisWindowRow) -> Self {
        self.source.insert(Self::DEFAULT_RUN, window_id, window);
        self
    }

    /// Arrangiert ein eingefrorenes Auswertungsfenster eines **bestimmten** Laufs — die Lage, in der
    /// mehrere Läufe dieselbe Historie paginiert haben (ADR-0016).
    #[must_use]
    pub fn with_window_of_run(
        mut self,
        run: &str,
        window_id: &str,
        window: AnalysisWindowRow,
    ) -> Self {
        self.source.insert(run, window_id, window);
        self
    }

    /// Lässt das **Lesen** terminal scheitern (io-Diagnose durchgereicht) — der eine fehlbare
    /// I/O-Schritt dieser Reichweite (ADR-0012).
    #[must_use]
    pub fn with_failing_source(mut self, message: &str) -> Self {
        self.source.fail_with(message);
        self
    }

    /// Fährt die echte Kette (Decoder → Handler) für eine **Lauf-Spanne** und gibt die Auswertung
    /// über die ganze Zeitachse zurück (`Err` = terminaler [`EvaluationError`]).
    ///
    /// Die Spanne wird dabei **so kodiert, wie `fenstern` sie publiziert** (`"{from}:{to}:{run}"`,
    /// unter dem Lauf [`DEFAULT_RUN`](Self::DEFAULT_RUN)) und
    /// durch den echten Decoder geschickt — der Spec prüft damit auch die Envelope-Form mit, nicht
    /// nur den Handler.
    ///
    /// # Panics
    /// Wenn die Spanne nicht decodiert — eine unlesbare Envelope ist eine io-Sorge, nicht
    /// Handler-Eingabe; die Specs reichen stets eine echte Spanne herein.
    pub fn run_history(&self, from: i64, to: i64) -> Result<HistoryEvaluationRow, EvaluationError> {
        self.run_history_of(Self::DEFAULT_RUN, from, to)
    }

    /// Wie [`run_history`](Self::run_history), aber für den Abschluss des Laufs `run` — kodiert, wie
    /// `fenstern` ihn publiziert (`"{from}:{to}:{run}"`).
    ///
    /// # Panics
    /// Wenn die Envelope nicht decodiert (leerer Lauf) — die Specs reichen stets einen echten herein.
    pub fn run_history_of(
        &self,
        run: &str,
        from: i64,
        to: i64,
    ) -> Result<HistoryEvaluationRow, EvaluationError> {
        let command = EvaluateHistoryCommandDecoder::decode(&format!("{from}:{to}:{run}"), "")
            .expect("Lauf-Spanne als Envelope");
        HistoryEvaluationHandler::new(
            &self.source,
            self.own_accounts.clone(),
            self.savings_threshold,
        )
        .handle(command)
    }
}

impl Default for HistoryTestApi {
    fn default() -> Self {
        Self::new()
    }
}
