use std::num::NonZeroU32;

use application_kernel::{CommandDecoder, Handler};
use fenstern_domain::WindowError;

use super::fakes::{FakeAnalysisWindowStore, FakeObfuscatedWindowSource, FakeWindowPublisher};
use crate::handler::WindowingHandler;
use crate::models::{AdvanceWindowCommandDecoder, AnalysisWindowRow, ObfuscatedRow};

/// Fluent-TestApi des fenstern-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md),
/// [ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)): die
/// **Eingangsfläche spricht nur DTOs** — mit [`with_obfuscated`](Self::with_obfuscated) arrangiert
/// man eine obfuskierte Zeile unter ihrem Buchungstag, [`run`](Self::run) fährt dann für einen
/// **Anker** die **echte** Kette über den `application_kernel`-Vertrag —
/// [`AdvanceWindowCommandDecoder`] (Envelope-Strings → Kommando, Shape Y) →
/// [`WindowingHandler`](crate::WindowingHandler) (Quell-/Repository-/Publisher-Bridge +
/// domänen-interner Handler) gegen öffentliche Fakes. **Keine Outbox-Mechanik:** der Slice ist eine
/// reine Pipeline; `OutboxConsumer`/`UnitOfWork` prüft der io-Integrationstest, nicht dieser Spec.
///
/// `run` gibt den **nackten Domänen-[`WindowError`]** zurück: eine leere Seite, gemischte Währungen,
/// ein fehlendes Pflichtfeld oder ein Fehlschlag beim Lesen/Ablegen/Publizieren lassen ihn
/// **terminal** scheitern.
///
/// Die drei `with_failing_*`-Schalter spielen den terminalen Fehlschlag der jeweiligen Richtung —
/// so trifft ein Spec **alle drei** fehlbaren I/O-Schritte black-box (ADR-0012).
pub struct WindowingTestApi {
    window_days: NonZeroU32,
    source: FakeObfuscatedWindowSource,
    store: FakeAnalysisWindowStore,
    publisher: FakeWindowPublisher,
}

impl WindowingTestApi {
    /// Neue TestApi mit Seitengröße `1` — reale Specs setzen sie über
    /// [`with_window_days`](Self::with_window_days).
    #[must_use]
    pub fn new() -> Self {
        Self {
            window_days: NonZeroU32::MIN,
            source: FakeObfuscatedWindowSource::default(),
            store: FakeAnalysisWindowStore::default(),
            publisher: FakeWindowPublisher::default(),
        }
    }

    /// Injiziert die Seitengröße (die io später aus der Config liest) — sie spannt zum Anker die
    /// Seite auf und entscheidet damit, welche Zeilen ins Fenster fallen.
    ///
    /// # Panics
    /// Wenn `days` null ist — eine Seitengröße null ist im Typ des Handlers unkonstruierbar; ein
    /// Spec, der sie arrangieren wollte, ist selbst falsch.
    #[must_use]
    pub fn with_window_days(mut self, days: u32) -> Self {
        self.window_days = NonZeroU32::new(days).expect("Seitengröße >= 1");
        self
    }

    /// Arrangiert eine obfuskierte Zeile unter ihrem Buchungstag in der (Fake-)`obfuskiert`-Ablage —
    /// die der Handler beim Lauf über die Seiten-Query liest.
    #[must_use]
    pub fn with_obfuscated(mut self, booking_day: i64, row: ObfuscatedRow) -> Self {
        self.source.insert(booking_day, row);
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

    /// Lässt das **Publizieren** terminal scheitern (io-Diagnose durchgereicht).
    #[must_use]
    pub fn with_failing_publisher(mut self, message: &str) -> Self {
        self.publisher.fail_with(message);
        self
    }

    /// Fährt die echte Kette (Decoder → Handler) für einen `AdvanceWindow`-**Anker** (der Tag-Stamp
    /// reist als `partition_key` der Envelope, kein Inhalt) und gibt das Handler-Ergebnis zurück
    /// (`Ok(())` = verarbeitet, `Err` = terminaler [`WindowError`]).
    ///
    /// # Panics
    /// Wenn der Anker nicht als Tag-Stamp decodiert — ein nicht-numerischer `partition_key` ist eine
    /// io-Envelope-Sorge, nicht Handler-Eingabe; die Specs reichen stets einen echten Anker herein.
    pub fn run(&self, anchor: i64) -> Result<(), WindowError> {
        let command = AdvanceWindowCommandDecoder::decode(&anchor.to_string(), "")
            .expect("Anker als Tag-Stamp");
        let handler =
            WindowingHandler::new(self.window_days, &self.source, &self.store, &self.publisher);
        handler.handle(command)
    }

    // --- Validierung des Ergebnisses ---------------------------------------------------------

    /// Prüft, dass **genau das** erwartete Auswertungsfenster in der Fenster-Tabelle liegt (ganze
    /// Zeile, ein Vergleich) — und damit zugleich, dass es **genau eines** ist (N→1).
    pub fn assert_stored(&self, expected: AnalysisWindowRow) {
        assert_eq!(self.store.windows(), vec![expected]);
    }

    /// Prüft, dass genau die erwarteten IDs unter dem Dispatch-Key `WindowClosed` als Zeiger
    /// publiziert wurden (Reihenfolge signifikant).
    pub fn assert_window_closed(&self, ids: &[&str]) {
        let erwartet: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
        assert_eq!(self.published_keys("WindowClosed"), erwartet);
    }

    /// Prüft, dass genau die erwarteten **Lauf-Spannen** unter dem Dispatch-Key `HistoryComplete`
    /// publiziert wurden — der Abschluss, der `auswerten`s fenster-übergreifende Reichweite auslöst.
    pub fn assert_history_complete(&self, spans: &[(i64, i64)]) {
        let erwartet: Vec<String> = spans
            .iter()
            .map(|(from, to)| format!("{from}:{to}"))
            .collect();
        assert_eq!(self.published_keys("HistoryComplete"), erwartet);
    }

    /// Die `partition_key`s der unter einem Dispatch-Key eingereihten Nachrichten, in
    /// Einreih-Reihenfolge.
    fn published_keys(&self, message_type: &str) -> Vec<String> {
        self.publisher
            .message_types()
            .into_iter()
            .zip(self.publisher.partition_keys())
            .filter(|(typ, _)| typ == message_type)
            .map(|(_, key)| key)
            .collect()
    }

    /// Prüft, dass **nichts** abgelegt oder publiziert wurde (terminaler Fehler vor der Ablage).
    pub fn assert_nothing_emitted(&self) {
        assert!(
            self.store.windows().is_empty(),
            "es wurde ein Fenster abgelegt"
        );
        assert!(
            self.publisher.partition_keys().is_empty(),
            "es wurde eine Nachricht publiziert"
        );
    }
}

impl Default for WindowingTestApi {
    fn default() -> Self {
        Self::new()
    }
}
