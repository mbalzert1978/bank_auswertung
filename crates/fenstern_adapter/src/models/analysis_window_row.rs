use fenstern_domain::AnalysisWindow;

/// Das **abgelegte Auswertungsfenster** als Zeile der eigenen Tabelle des Slice — das Ausgangs-DTO,
/// das die `RepositoryBridge` in den Speicher-Seam staged. Es
/// trägt die Lauf-Spanne, die eine Währung des Fensters und die **Fremdschlüssel** auf genau die
/// obfuskierten Zeilen, die es einfriert (ADR-0013, Entscheidung A: FKs, keine eingebettete
/// Nutzlast). Seine **eigene** ID vergibt die Persistenz beim Ablegen — sie steht darum nicht hier
/// drin.
///
/// **Datenschutz:** stromabwärts der Obfuskierungs-Grenze; die Zeile trägt nur opake IDs, eine
/// Währung und Tag-Stamps — keinen strukturellen Identifikator und keinen Freitext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisWindowRow {
    pub from: i64,
    pub to: i64,
    pub currency: String,
    pub booking_ids: Vec<String>,
}

/// Projiziert das **Domänen-Aggregat** auf die Zeile — die einzige Stelle, an der es die
/// Adapter-Grenze überquert (die Repository-Bridge ruft sie vor dem Ablegen).
impl From<&AnalysisWindow> for AnalysisWindowRow {
    fn from(window: &AnalysisWindow) -> Self {
        let span = window.span();
        Self {
            from: span.from().value(),
            to: span.to().value(),
            currency: window.currency().as_str().to_string(),
            booking_ids: window
                .bookings()
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
        }
    }
}
