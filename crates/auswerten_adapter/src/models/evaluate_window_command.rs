use application_kernel::{Command, CommandDecoder};
use auswerten_domain::WindowId;

/// Der Dispatch-Key der **konsumierten** Nachricht — auf ihn löst `io`s Prozessor diesen Slice als
/// Consumer auf. Prosa-Name im ADR: **`FensterGeschlossen`**, publiziert von `fenstern`, sobald es
/// ein Auswertungsfenster materialisiert und abgelegt hat.
pub const CONSUMED_MESSAGE_TYPE: &str = "WindowClosed";

/// Das interne Kommando am Slice-Eingang: es trägt **nur die ID** des eingefrorenen
/// Auswertungsfensters — einen **Zeiger**, kein eingebettetes Aggregat (uniformes ID-Handoff,
/// ADR-0012). Der [`WindowEvaluationHandler`](crate::WindowEvaluationHandler) baut daraus das
/// domänen-interne `EvaluateWindow`. Es erfüllt `application_kernel::Command` (Marker).
pub struct EvaluateWindowCommand {
    window_id: String,
}

impl EvaluateWindowCommand {
    /// Die ID als Domänen-[`WindowId`]. Unfehlbar: der Decoder hat die Leere schon abgewiesen, was
    /// hier ankommt, ist ein echter Zeiger.
    pub(crate) fn window(&self) -> WindowId {
        WindowId::new(self.window_id.clone())
    }
}

impl Command for EvaluateWindowCommand {}

/// Der slice-gelieferte [`CommandDecoder`] (reine ACL, Shape Y): `io` extrahiert die Envelope und
/// reicht nur die nackten Strings herein — der Slice nennt **keinen** Outbox-Typ. Die Fenster-ID
/// reist im `partition_key` (wie `obfuskieren` die Buchungs-ID trägt); der `content` ist leer,
/// **kein serde**.
///
/// Ein `partition_key`, der **keine** ID trägt (leer oder nur Whitespace), ist kein gültiger Zeiger
/// → `None`, das `io` auf der Envelope-Ebene behandelt (nicht als Handler-Fehler durchgereicht).
/// **Parse, don't validate:** was hier durchkommt, ist eine echte, nicht-leere ID — der Handler
/// bekommt nie einen Leerwert, den er noch prüfen müsste.
pub struct EvaluateWindowCommandDecoder;

impl CommandDecoder for EvaluateWindowCommandDecoder {
    type Command = EvaluateWindowCommand;

    fn decode(partition_key: &str, _content: &str) -> Option<Self::Command> {
        let trimmed = partition_key.trim();
        (!trimmed.is_empty()).then(|| EvaluateWindowCommand {
            window_id: trimmed.to_string(),
        })
    }
}
