use application_kernel::{Command, PublishCommand};

/// Der Dispatch-Key der **Abschluss**-Nachricht. Sprach-Konvention wie bei `Ingested`/`Obfuscated`:
/// der Code-Name ist englisch (`HistoryComplete`), der Prosa-Begriff des ADR bleibt
/// „HistorieVollständig".
pub(crate) const MESSAGE_TYPE: &str = "HistoryComplete";

/// Das ausgehende **Publish-Command** der `HistoryComplete`-Nachricht: die **Lauf-Spanne**, mit der
/// `auswerten` seine fenster-übergreifende Query auf genau diesen Lauf scopt (ADR-0013,
/// Entscheidung E).
///
/// Spanne und Lauf reisen als `partition_key` in der Form `"{from}:{to}:{run}"` — **kein serde**,
/// wie überall im Slice. Der **Lauf** ist der eigentliche Scope (ADR-0016): jeder Lauf paginiert
/// dieselbe Historie, die Spanne allein grenzt Läufe nicht gegeneinander ab. Der Typ ist **öffentlich, aber
/// opak** (private Felder, crate-interner Konstruktor): der Seam-Vertrag nennt ihn als Nutzlast,
/// gebaut wird er nur hier im Adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCompleteMessage {
    span: String,
}

impl HistoryCompleteMessage {
    /// Trägt die Lauf-Spanne (beide Grenzen einschließlich) und den Lauf.
    pub(crate) fn from_span(from: i64, to: i64, run: &str) -> Self {
        Self {
            span: format!("{from}:{to}:{run}"),
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
