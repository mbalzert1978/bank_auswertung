use crate::entities::RunHistory;
use crate::error::EvaluationError;
use crate::value_objects::RunSpan;

/// Der von der Domäne **verlangte** Lese-Port der **fenster-übergreifenden** Reichweite: er liefert
/// die [`RunHistory`] — die Buchungen **aller** Auswertungsfenster **eines** Laufs samt dem
/// Seed-Marker ihrer Tokenisierung —, gescopt über die [`RunSpan`] (ADR-0013, Entscheidung E,
/// Reichweite 2).
///
/// **Der Seed-Marker reist mit**, weil die Wiederkehr-Erkennung Umbuchungen auf Token herausnettet
/// und dafür — wie die pro-Fenster-Reichweite — prüfen muss, dass Historie und Eigene-Konten-Config
/// aus derselben Tokenisierung stammen (ADR-0010).
///
/// **Eine Query, nicht N+1:** die Domäne verlangt *eine* Sicht über die ganze Zeitachse; dass io sie
/// aus vielen Fenster-Zeilen und ihrem Join auf die `obfuskiert`-Ablage zusammensetzt, ist
/// Beschaffungs-Mechanik — dieselbe Linie, die
/// [`AnalysisWindowSource`](crate::AnalysisWindowSource) beim Join zieht. Ein Port „gib mir die
/// Fenster-IDs, dann hol ich jedes einzeln" trüge die Zerlegung der Ablage in die Domäne.
///
/// **Ein reines Read, ohne Gegenstück:** es gibt **keinen** Schreib-Port dazu und **kein**
/// Vorkommens-/Fold-Log. Die Aggregate sind die alleinige Wahrheit, die fenster-übergreifende Sicht
/// entsteht bei jeder Query neu aus unveränderlichen Snapshots — daher ist sie von Natur aus
/// deterministisch, auch bei erneuter Zustellung, und braucht keine Dedupe-/Idempotenz-Maschinerie.
///
/// **Fehlbar und terminal** ([`EvaluationError`]): ein Fehlschlag der Beschaffung endet den Lauf.
/// Ein **leeres** Ergebnis ist dagegen kein Fehler — ein Lauf ohne Buchung in der Spanne ist eine
/// gültige Auskunft, nicht ein Fehlschlag.
pub trait OccurrenceSource {
    /// Liest die Historie aller Fenster **innerhalb** der Lauf-Spanne, oder scheitert terminal.
    fn history_in(&self, span: RunSpan) -> Result<RunHistory, EvaluationError>;
}
