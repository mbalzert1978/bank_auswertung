use std::num::NonZeroU32;

use application_kernel::{CommandDecoder, Handler};
use fenstern_domain::WindowError;

use super::fakes::{FakeAnalysisWindowStore, FakeObfuscatedWindowSource, FakeWindowPublisher};
use crate::handler::WindowingHandler;
use crate::models::{
    AdvanceWindowCommandDecoder, AnalysisWindowRow, CONSUMED_MESSAGE_TYPE, ObfuscatedRow,
    StartAnalysisCommandDecoder,
};
use crate::start_handler::StartHandler;

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
/// Die Schranke, ab der [`run_chain`](WindowingTestApi::run_chain) einen nicht terminierenden Lauf
/// als Spec-Fehlschlag meldet, statt zu hängen. Großzügig über jeder realen Seitenzahl (~3 bei drei
/// Jahren Historie) — sie greift nur, wenn der streng monoton sinkende Anker gebrochen ist.
const KETTEN_SCHRANKE: usize = 1_000;

pub struct WindowingTestApi {
    window_days: NonZeroU32,
    run: String,
    source: FakeObfuscatedWindowSource,
    store: FakeAnalysisWindowStore,
    publisher: FakeWindowPublisher,
}

impl WindowingTestApi {
    /// Die Lauf-Identität, unter der die TestApi anstößt, solange ein Spec keine eigene setzt
    /// ([`with_run`](Self::with_run)).
    pub const DEFAULT_RUN: &str = "lauf-1";

    /// Neue TestApi mit Seitengröße `1` — reale Specs setzen sie über
    /// [`with_window_days`](Self::with_window_days).
    #[must_use]
    pub fn new() -> Self {
        Self {
            window_days: NonZeroU32::MIN,
            run: Self::DEFAULT_RUN.to_string(),
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

    /// Setzt die **Lauf-Identität**, unter der [`start`](Self::start) bzw. [`run`](Self::run) den
    /// Lauf führen (ADR-0016) — sonst [`DEFAULT_RUN`](Self::DEFAULT_RUN). Vergeben wird sie in Produktion von dem, der
    /// anstößt; der Slice reicht sie nur durch.
    #[must_use]
    pub fn with_run(mut self, run: &str) -> Self {
        self.run = run.to_string();
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

    /// Fährt den **Anstoß**: die echte Kette [`StartAnalysisCommandDecoder`] →
    /// [`StartHandler`](crate::StartHandler) mit dem Auslöser-Kommando, das nur die Lauf-Identität
    /// trägt. Der Anker wird
    /// dabei nicht hereingereicht, sondern vom Handler über den Anker-Port beschafft — die Specs
    /// arrangieren ihn allein über [`with_obfuscated`](Self::with_obfuscated), **ohne Uhr**.
    ///
    /// `Ok(())` = verarbeitet (auch beim **leeren** Bestand, dann ohne jede Nachricht);
    /// `Err` = terminaler [`WindowError`].
    pub fn start(&self) -> Result<(), WindowError> {
        let command = StartAnalysisCommandDecoder::decode(&self.run, "")
            .expect("Lauf-Identität als Schlüssel");
        StartHandler::new(&self.source, &self.publisher).handle(command)
    }

    /// Fährt **einen** Fenster-Schritt der echten Kette (Decoder → Handler) für einen Anker und gibt
    /// das Handler-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` = terminaler [`WindowError`]). Der
    /// Anker ist zugleich der Lauf-Gipfel — genau die Lage, in der ein Lauf beginnt.
    ///
    /// Für den **mehrseitigen** Lauf ist [`run_chain`](Self::run_chain) zuständig: `run` verarbeitet
    /// bewusst nur das eine Kommando und lässt die publizierte Fortsetzung liegen, damit ein Spec
    /// den einzelnen Schritt isoliert betrachten kann.
    pub fn run(&self, anchor: i64) -> Result<(), WindowError> {
        self.run_key(&format!("{anchor}:{anchor}:{}", self.run))
    }

    /// Fährt den **ganzen Lauf**: den ersten Schritt zum Anker und danach jede vom Slice selbst
    /// publizierte Fortsetzung, bis die Historie versiegt (ADR-0013, Entscheidung B,
    /// Präzisierung 2). Sie spielt damit die Rolle, die in Produktion die Outbox hat — der Slice
    /// selbst kennt keine Schleife, er reiht nur sein nächstes Kommando ein.
    ///
    /// Ein terminaler Fehler bricht den Lauf sofort ab und wird nackt zurückgegeben.
    ///
    /// # Panics
    /// Wenn die Kette nicht binnen `KETTEN_SCHRANKE` Schritten terminiert — der Anker muss streng
    /// monoton sinken, ein Nicht-Terminieren ist ein **Fehlschlag des Specs**, kein Hänger.
    pub fn run_chain(&self, anchor: i64) -> Result<(), WindowError> {
        self.fahre_kette(format!("{anchor}:{anchor}:{}", self.run), 0)
    }

    /// Fährt den **ganzen Lauf ab dem Anstoß**: erst [`start`](Self::start) (der den Initial-Anker
    /// selbst beschafft), dann jede vom Slice publizierte Fortsetzung — die einzige Sicht, in der
    /// der vom Anstoß gesetzte **Lauf-Gipfel** bis zur publizierten Lauf-Spanne beobachtbar wird.
    ///
    /// Auf **leerem** Bestand publiziert der Anstoß nichts; dann endet der Lauf sofort mit `Ok(())`,
    /// ohne dass je ein Fenster-Schritt läuft.
    ///
    /// # Panics
    /// Wenn die Kette nicht binnen `KETTEN_SCHRANKE` Schritten terminiert.
    pub fn start_chain(&self) -> Result<(), WindowError> {
        self.start()?;
        // Der Anstoß hat die **nullte** Fortsetzung publiziert — oder gar keine (leerer Bestand).
        match self.naechste_fortsetzung(0) {
            Some(key) => self.fahre_kette(key, 1),
            None => Ok(()),
        }
    }

    /// Der gemeinsame Ketten-Motor von [`run_chain`](Self::run_chain) und
    /// [`start_chain`](Self::start_chain): verarbeitet ein Kommando nach dem anderen, bis der Slice
    /// keine Fortsetzung mehr publiziert. Er spielt die Rolle, die in Produktion die Outbox hat —
    /// der Slice selbst kennt keine Schleife. `bereits` ist die Zahl der schon publizierten
    /// Fortsetzungen, ab der die nächste erwartet wird.
    fn fahre_kette(&self, erster_key: String, bereits: usize) -> Result<(), WindowError> {
        let mut key = erster_key;
        let mut fortsetzungen = bereits;
        loop {
            self.run_key(&key)?;
            let Some(naechste) = self.naechste_fortsetzung(fortsetzungen) else {
                return Ok(());
            };
            fortsetzungen += 1;
            key = naechste;
        }
    }

    /// Die `fortsetzungen`-te vom Slice publizierte Fortsetzung, oder `None`, wenn die Kette
    /// terminiert hat.
    ///
    /// # Panics
    /// Wenn die Kette nicht binnen [`KETTEN_SCHRANKE`] Schritten terminiert.
    fn naechste_fortsetzung(&self, bereits: usize) -> Option<String> {
        let naechste = self
            .published_keys(CONSUMED_MESSAGE_TYPE)
            .get(bereits)
            .cloned();
        assert!(
            naechste.is_none() || bereits < KETTEN_SCHRANKE,
            "die Kette terminiert nicht: nach {KETTEN_SCHRANKE} Schritten wird immer noch fortgesetzt"
        );
        naechste
    }

    /// Decodiert einen rohen `partition_key` und fährt den Handler damit — die eine Stelle, an der
    /// die Specs die echte `Decoder → Handler`-Kette betreten.
    ///
    /// # Panics
    /// Wenn der Schlüssel nicht als `"{anchor}:{run_top}:{run}"` decodiert — eine unlesbare Envelope ist
    /// eine io-Sorge, nicht Handler-Eingabe; die Specs reichen stets einen echten Anker herein.
    fn run_key(&self, partition_key: &str) -> Result<(), WindowError> {
        let command =
            AdvanceWindowCommandDecoder::decode(partition_key, "").expect("Anker als Tag-Stamp");
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

    /// Prüft, dass **genau diese** Auswertungsfenster in der Fenster-Tabelle liegen, in
    /// Ablage-Reihenfolge — die Sicht für den mehrseitigen Lauf, in dem je Seite eines entsteht.
    pub fn assert_stored_windows(&self, expected: &[AnalysisWindowRow]) {
        assert_eq!(self.store.windows(), expected);
    }

    /// Prüft, dass genau die erwarteten **Anker** als Selbst-Fortsetzung publiziert wurden
    /// (Reihenfolge signifikant) — ein leeres Erwartungsfeld heißt „die Kette hat sich nicht
    /// fortgesetzt". Der mitreisende Lauf-Gipfel ist über die publizierte Lauf-Spanne belegt.
    pub fn assert_advance_window(&self, anchors: &[i64]) {
        let erwartet: Vec<String> = anchors.iter().map(i64::to_string).collect();
        let publiziert: Vec<String> = self
            .published_keys(CONSUMED_MESSAGE_TYPE)
            .iter()
            .map(|key| key.split(':').next().unwrap_or_default().to_string())
            .collect();
        assert_eq!(publiziert, erwartet);
    }

    /// Prüft, dass genau die erwarteten IDs unter dem Dispatch-Key `WindowClosed` als Zeiger
    /// publiziert wurden (Reihenfolge signifikant).
    pub fn assert_window_closed(&self, ids: &[&str]) {
        let erwartet: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
        assert_eq!(self.published_keys("WindowClosed"), erwartet);
    }

    /// Prüft, dass genau die erwarteten **Lauf-Spannen** unter dem Dispatch-Key `HistoryComplete`
    /// publiziert wurden — der Abschluss, der `auswerten`s fenster-übergreifende Reichweite auslöst
    /// —, jede unter der Lauf-Identität dieser TestApi.
    pub fn assert_history_complete(&self, spans: &[(i64, i64)]) {
        let erwartet: Vec<String> = spans
            .iter()
            .map(|(from, to)| format!("{from}:{to}:{}", self.run))
            .collect();
        assert_eq!(self.published_keys("HistoryComplete"), erwartet);
    }

    /// Prüft, dass **jede** publizierte Fortsetzung (`AdvanceWindow`) und jeder Abschluss
    /// (`HistoryComplete`) die Lauf-Identität `run` trägt — sie reist unverändert durch die ganze
    /// Kette (ADR-0016).
    pub fn assert_every_message_carries_run(&self, run: &str) {
        let schluessel: Vec<String> = [CONSUMED_MESSAGE_TYPE, "HistoryComplete"]
            .into_iter()
            .flat_map(|typ| self.published_keys(typ))
            .collect();
        assert!(!schluessel.is_empty(), "es wurde nichts publiziert");
        for key in schluessel {
            assert_eq!(key.rsplit(':').next(), Some(run), "{key}");
        }
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
