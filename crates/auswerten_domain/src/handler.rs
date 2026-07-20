use std::collections::BTreeSet;

use crate::entities::{AnalysisWindow, WindowEvaluation};
use crate::error::EvaluationError;
use crate::models::EvaluateWindow;
use crate::ports::{AnalysisWindowSource, Categorizer, CategoryRegistry, EvaluationRepository};
use crate::value_objects::{CategoryAnswer, OwnAccounts};

/// Orchestriert die **pro-Fenster-Reichweite** (ADR-0013, Entscheidung E, Reichweite 1) — er **lebt
/// in der Domäne** und hängt **nur** an den von ihr verlangten Ports; die Verdrahtung an die
/// Außenwelt (io/Fakes) besorgt der Adapter über Port-Bridges.
///
/// **Ablauf je Kommando** — die uniforme Handler-Form aller Slices:
/// 1. das eingefrorene Auswertungsfenster **per ID lesen** ([`AnalysisWindowSource`]) — herein kam
///    ein **Zeiger**, kein eingebettetes Aggregat (ADR-0012),
/// 2. die **unbekannten** Buchungsarten beim Orakel anfragen ([`CategoryRegistry`] +
///    [`Categorizer`], siehe `categorise`),
/// 3. daraus die periodische Kennzahl **innerhalb** des Fensters rechnen (Cashflow je Monat),
/// 4. das Ergebnis **ablegen** ([`EvaluationRepository`]).
///
/// **Warum das Orakel hier hängt und nicht an einem eigenen Consumer:** der Aufruf ist eine
/// **Domänen-Entscheidung**, kein Pipeline-Schritt (ADR-0004). Ein eigener Schritt mit eigenem
/// Outbox-Kommando machte aus „die Analyse fragt nach, wenn sie etwas nicht kennt" ein
/// Infrastruktur-Ereignis — und damit das LLM zum Kategorisierer-of-record statt zum Orakel.
///
/// **Kein Publish:** diese Reichweite stößt nichts stromabwärts an. Die zweite, fenster-übergreifende
/// Reichweite hängt nicht an ihr, sondern an `fenstern`s `HistorieVollständig` — genau deshalb, weil
/// nur `fenstern` die Erschöpfung der Historie kennt. Eine Nachricht ohne Empfänger zu publizieren
/// wäre Mechanik auf Vorrat.
///
/// Der Handler kennt **keine** Transaktion: die Ablage *staged* nur, den atomaren Commit besorgt io.
/// Lesen und Ablegen sind je fehlbar und terminal; ihr [`EvaluationError`] propagiert per `?` — kein
/// Dead-Letter, kein Retry (ADR-0012).
pub struct WindowEvaluationHandler<'a> {
    source: &'a dyn AnalysisWindowSource,
    repository: &'a dyn EvaluationRepository,
    /// Was die Domäne bereits **kennt** — die Voraussetzung dafür, dass „unbekannt" überhaupt eine
    /// Bedeutung hat.
    registry: &'a dyn CategoryRegistry,
    /// Das **Orakel** für den Rest.
    categorizer: &'a dyn Categorizer,
    /// Die Eigene-Konten-Config als **Wert** (ADR-0013, Entscheidung F): sie steht beim Bau des
    /// Handlers fest und gilt für den ganzen Lauf — sie ist Konfiguration, kein Port. Die Domäne
    /// liest darum **keine Datei**; wer sie beschafft, bleibt io überlassen.
    own_accounts: &'a OwnAccounts,
}

impl<'a> WindowEvaluationHandler<'a> {
    pub fn new(
        source: &'a dyn AnalysisWindowSource,
        repository: &'a dyn EvaluationRepository,
        registry: &'a dyn CategoryRegistry,
        categorizer: &'a dyn Categorizer,
        own_accounts: &'a OwnAccounts,
    ) -> Self {
        Self {
            source,
            repository,
            registry,
            categorizer,
            own_accounts,
        }
    }

    /// Verarbeitet das Kommando: Fenster per ID lesen → Cashflow je Monat rechnen → (staged)
    /// ablegen. `Ok(())` = verarbeitet; `Err(EvaluationError)` = terminaler Fehler (der Adapter
    /// reicht ihn an io).
    pub fn handle(&self, command: EvaluateWindow) -> Result<(), EvaluationError> {
        let window = self.source.read(command.window())?;
        // **Vor** der Auswertung: stammen Fenster und Eigen-Menge aus derselben Tokenisierung?
        // Nur dann trägt der Token-Vergleich. Stimmen die Marker nicht überein, ist das terminal —
        // sonst liefe jede Umbuchung lautlos als „fremd" durch (ADR-0010).
        if window.seed_marker() != self.own_accounts.marker() {
            return Err(EvaluationError::SeedMismatch);
        }
        let categories = self.categorise(&window)?;
        self.repository.save(&WindowEvaluation::of(
            command.into_window(),
            &window,
            self.own_accounts,
            categories,
        ))
    }

    /// Fragt das Orakel — **nur** nach dem, was die Domäne nicht kennt (ADR-0004).
    ///
    /// Zwei Filter hintereinander, und beide sind fachlich, nicht mechanisch: das Fenster liefert
    /// je **Buchungsart** eine Anfrage (dedupliziert über den Fingerabdruck), und davon bleibt, was
    /// das **Register** nicht schon weiß. Ein bekanntes Etikett erneut zu erfragen wäre nicht bloß
    /// verschwenderisch — es stellte die gelernte Korrektur zur Disposition, die genau deshalb
    /// gelernt wurde, weil das Orakel danebenlag.
    ///
    /// **Bleibt nichts übrig, wird nicht gefragt** — und zwar an *beiden* Stellen: hat das Fenster
    /// gar keine Buchungsart mit Kategorie-Signal, wird schon das **Register** nicht befragt; bleibt
    /// nach dem Abgleich nichts Unbekanntes, bleibt der **Chat** stumm. Ein Aufruf mit leerer Liste
    /// wäre in beide Richtungen eine Frage ohne Gegenstand.
    ///
    /// Beim Register ist das mehr als Sparsamkeit: die Auskunft ist **terminal**. Ein ausgefallenes
    /// Register beendete sonst einen Lauf, der gar nichts von ihm wollte — ein Fehlschlag ohne
    /// Ursache in der Sache.
    fn categorise(&self, window: &AnalysisWindow) -> Result<Vec<CategoryAnswer>, EvaluationError> {
        let requests = window.category_requests();
        if requests.is_empty() {
            return Ok(Vec::new());
        }
        let known = self
            .registry
            .known_among(
                &requests
                    .iter()
                    .map(|request| request.fingerprint().clone())
                    .collect::<Vec<_>>(),
            )?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let unknown = requests
            .into_iter()
            .filter(|request| !known.contains(request.fingerprint()))
            .collect::<Vec<_>>();
        match unknown.is_empty() {
            true => Ok(Vec::new()),
            false => self.categorizer.ask(&unknown),
        }
    }
}
