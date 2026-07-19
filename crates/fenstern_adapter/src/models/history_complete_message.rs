use application_kernel::{Command, PublishCommand};

/// Der Dispatch-Key der **Abschluss**-Nachricht. Sprach-Konvention wie bei `Ingested`/`Obfuscated`:
/// der Code-Name ist englisch (`HistoryComplete`), der Prosa-Begriff des ADR bleibt
/// „HistorieVollständig".
pub(crate) const MESSAGE_TYPE: &str = "HistoryComplete";

/// Das ausgehende **Publish-Command** der `HistoryComplete`-Nachricht: die **Lauf-Spanne**, mit der
/// `auswerten` seine fenster-übergreifende Query auf genau diesen Lauf scopt (ADR-0013,
/// Entscheidung E).
///
/// Die Spanne reist als `partition_key` in der Form `"{from}:{to}"` — **kein serde**, wie überall im
/// Slice; die beiden Tag-Stamps sind alles, was die Nachricht trägt. Der Typ ist **öffentlich, aber
/// opak** (private Felder, crate-interner Konstruktor): der Seam-Vertrag nennt ihn als Nutzlast,
/// gebaut wird er nur hier im Adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCompleteMessage {
    span: String,
}

impl HistoryCompleteMessage {
    /// Trägt die Lauf-Spanne (beide Grenzen einschließlich).
    pub(crate) fn from_span(from: i64, to: i64) -> Self {
        Self {
            span: format!("{from}:{to}"),
        }
    }
}

impl Command for HistoryCompleteMessage {}

impl PublishCommand for HistoryCompleteMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.span
    }
}
