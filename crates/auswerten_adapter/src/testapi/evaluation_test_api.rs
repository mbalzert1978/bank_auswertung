use application_kernel::{CommandDecoder, Handler};
use auswerten_domain::EvaluationError;

use super::fakes::{
    FakeAnalysisWindowSource, FakeCategorizer, FakeCategoryRegistry, FakeEvaluationStore,
};
use crate::handler::WindowEvaluationHandler;
use crate::models::{
    AnalysisWindowRow, CategoryRequestRow, EvaluateWindowCommandDecoder, OwnAccountsConfig,
    WindowEvaluationRow,
};

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
    registry: FakeCategoryRegistry,
    categorizer: FakeCategorizer,
    own_accounts: OwnAccountsConfig,
}

impl EvaluationTestApi {
    /// Neue TestApi mit leerer Fenster-Ablage, leerer Ergebnis-Tabelle und **noch nicht
    /// arrangierter** Eigene-Konten-Config.
    ///
    /// **[`with_own_accounts`](Self::with_own_accounts) ist Pflicht, nicht Kür.** Die Nullwert-Config
    /// trägt keinen Seed-Marker, und ein fehlender Marker ist ein fehlendes Pflichtfeld — ein
    /// [`run`](Self::run) darauf scheitert terminal, **bevor** irgendetwas gelesen wird. Das ist
    /// gewollt: eine Eigen-Menge ohne Kennzeichen ihrer Tokenisierung ist keine gültige Eingabe,
    /// und die TestApi darf sie nicht zu einer machen. (Dieselbe Linie zieht
    /// [`AnalysisWindowRow`]`::default()` mit ihrer leeren Währung — ein `Default` ist hier der
    /// **Nullwert** des DTOs, nicht ein gültiger Wert.)
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: FakeAnalysisWindowSource::default(),
            store: FakeEvaluationStore::default(),
            registry: FakeCategoryRegistry::default(),
            categorizer: FakeCategorizer::default(),
            own_accounts: OwnAccountsConfig::default(),
        }
    }

    /// Arrangiert Fingerabdrücke als der Domäne **bereits bekannt** — als hätte ein früherer Lauf
    /// ihr Etikett gelernt. Genau sie dürfen das Orakel nicht mehr erreichen.
    #[must_use]
    pub fn with_known_categories(mut self, fingerprints: &[&str]) -> Self {
        for fingerprint in fingerprints {
            self.registry.insert(fingerprint);
        }
        self
    }

    /// Arrangiert das **Vokabular des Orakels**: welche Kategorie es zu welchem Fingerabdruck
    /// liefern würde. Wonach nicht gefragt wird, kann es nicht beantworten — und wonach gefragt
    /// wird, ohne dass es hier steht, bleibt unbeantwortet.
    #[must_use]
    pub fn with_oracle(mut self, vocabulary: &[(&str, &str)]) -> Self {
        for (fingerprint, category) in vocabulary {
            self.categorizer.knows(fingerprint, category);
        }
        self
    }

    /// Reicht die **Eigene-Konten-Config als Wert** herein — genau die Lage des Handlers in
    /// Produktion: io verdrahtet sie beim Bau, der Slice liest **keine Datei**. Die Token sind
    /// bereits mit demselben Seed gebildet wie die Gegenkonto-Token der Buchungen.
    ///
    /// Ihr **Seed-Marker muss der des arrangierten Fensters sein** — sonst endet der Lauf terminal
    /// mit `SeedMismatch`, und genau das ist der Zweck des Markers. Eine Config **ohne** eigene
    /// Konten (aber **mit** Marker) ist dagegen eine gültige Lage: dann ist jedes Gegenkonto fremd.
    #[must_use]
    pub fn with_own_accounts(mut self, own_accounts: OwnAccountsConfig) -> Self {
        self.own_accounts = own_accounts;
        self
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

    /// Arrangiert Etiketten, die das Orakel **ungefragt** mitschickt — zu Fingerabdrücken, nach
    /// denen nie gefragt wurde. Damit lässt sich das übergriffige Orakel spielen: eines, das
    /// Schlüssel *erfindet* statt zu antworten.
    #[must_use]
    pub fn with_rogue_oracle_answers(mut self, answers: &[(&str, &str)]) -> Self {
        for (fingerprint, category) in answers {
            self.categorizer.answers_unasked(fingerprint, category);
        }
        self
    }

    /// Lässt die Auskunft des **Kategorie-Registers** terminal scheitern.
    #[must_use]
    pub fn with_failing_registry(mut self, message: &str) -> Self {
        self.registry.fail_with(message);
        self
    }

    /// Lässt die Anfrage ans **Orakel** terminal scheitern.
    #[must_use]
    pub fn with_failing_categorizer(mut self, message: &str) -> Self {
        self.categorizer.fail_with(message);
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
        WindowEvaluationHandler::new(
            &self.source,
            &self.store,
            &self.registry,
            &self.categorizer,
            self.own_accounts.clone(),
        )
        .handle(command)
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

    /// Prüft, dass **genau diese** Anfragen ans Orakel gingen — in Reihenfolge und vollständig.
    ///
    /// Der Vergleich ist bewusst **die ganze Liste** statt einer Enthaltensein-Prüfung: nur so
    /// belegt er zugleich, dass **nichts darüber hinaus** hinausging — und das ist bei dem einen
    /// DTO, das die Maschine verlässt, die interessantere Hälfte der Zusage.
    pub fn assert_asked(&self, expected: Vec<CategoryRequestRow>) {
        assert_eq!(self.categorizer.asked(), expected);
    }

    /// Prüft, dass das Orakel **gar nicht** behelligt wurde — kein Egress, kein Chat.
    pub fn assert_nothing_asked(&self) {
        assert!(
            self.categorizer.asked().is_empty(),
            "es ging eine Anfrage ans Orakel"
        );
    }
}

impl Default for EvaluationTestApi {
    fn default() -> Self {
        Self::new()
    }
}
