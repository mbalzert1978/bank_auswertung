use application_kernel::{Command, CommandDecoder};
use fenstern_domain::DayStamp;

/// Der Dispatch-Key der **konsumierten** Nachricht — auf ihn löst `io`s Prozessor diesen Slice als
/// Consumer auf. Prosa-Name im ADR: der **Fenster-Schritt**.
pub const CONSUMED_MESSAGE_TYPE: &str = "AdvanceWindow";

/// Das interne Kommando am Slice-Eingang: es trägt **immer** einen Anker (ADR-0013, Entscheidung B)
/// — den Tag-Stamp, der die **obere** Grenze der Seite bildet. Der
/// [`WindowingHandler`](crate::WindowingHandler) baut daraus das domänen-interne `AdvanceWindow`.
/// Es erfüllt `application_kernel::Command` (Marker).
pub struct AdvanceWindowCommand {
    anchor: i64,
}

impl AdvanceWindowCommand {
    /// Der Anker als Tag-Stamp, den der Handler in einen `DayStamp` hüllt.
    pub(crate) fn anchor(&self) -> DayStamp {
        DayStamp::new(self.anchor)
    }
}

impl Command for AdvanceWindowCommand {}

/// Der slice-gelieferte [`CommandDecoder`] (reine ACL, Shape Y): `io` extrahiert die Envelope und
/// reicht nur die nackten Strings herein — der Slice nennt **keinen** Outbox-Typ. Der Anker reist
/// als **stringifizierter Tag-Stamp im `partition_key`** (ADR-0013, Entscheidung B: kein serde);
/// der `content` ist leer.
///
/// Ein `partition_key`, der **kein** Tag-Stamp ist (leer, blank, keine Zahl), ist kein gültiger
/// Anker → `None`, das `io` auf der Envelope-Ebene behandelt (nicht als Handler-Fehler
/// durchgereicht). **Parse, don't validate:** was hier durchkommt, ist ein echter `i64` — der
/// Handler bekommt nie einen Text, den er noch deuten müsste.
pub struct AdvanceWindowCommandDecoder;

impl CommandDecoder for AdvanceWindowCommandDecoder {
    type Command = AdvanceWindowCommand;

    fn decode(partition_key: &str, _content: &str) -> Option<Self::Command> {
        partition_key
            .trim()
            .parse::<i64>()
            .ok()
            .map(|anchor| AdvanceWindowCommand { anchor })
    }
}
