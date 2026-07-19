use application_kernel::{Command, PublishCommand};

/// Der Dispatch-Key der ausgehenden Nachricht **stromabwärts nach `auswerten`**. Sprach-Konvention
/// wie bei `Ingested`/`Obfuscated`: der Code-Name ist englisch (`WindowClosed`), der Prosa-Begriff
/// des ADR bleibt „FensterGeschlossen".
pub(crate) const MESSAGE_TYPE: &str = "WindowClosed";

/// Das ausgehende **Publish-Command** der `WindowClosed`-Nachricht: ein reiner **Zeiger** auf das in
/// der Fenster-Tabelle abgelegte Aggregat — nur die von der Ablage vergebene `id`, als
/// `partition_key` der Nachricht (Tabelle = Wahrheit, Message = Pointer).
///
/// Es erfüllt `application_kernel::PublishCommand`; **kein serde** — die ID reist in der
/// Outbox-Hülle. Der Typ ist **öffentlich, aber opak** (private Felder, crate-interner Konstruktor):
/// der Seam-Vertrag nennt ihn als Nutzlast, gebaut wird er nur hier im Adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowClosedMessage {
    id: String,
}

impl WindowClosedMessage {
    /// Zeigt auf das abgelegte Fenster (übernimmt nur seine ID).
    pub(crate) fn from_id(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl Command for WindowClosedMessage {}

impl PublishCommand for WindowClosedMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.id
    }
}
