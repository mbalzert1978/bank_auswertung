use application_kernel::{Command, CommandDecoder};
use auswerten_domain::{DayStamp, RunSpan};

/// Der Dispatch-Key der **konsumierten** Nachricht dieser Reichweite — auf ihn löst `io`s Prozessor
/// den fenster-übergreifenden Handler auf. Prosa-Name im ADR: **`HistorieVollständig`**, publiziert
/// von `fenstern`, sobald die Historie erschöpft ist.
///
/// Er ist **ein anderer** als
/// [`CONSUMED_MESSAGE_TYPE`](crate::CONSUMED_MESSAGE_TYPE) (`"WindowClosed"`) — und genau darin
/// liegt die Abgrenzung der beiden Reichweiten: ein `FensterGeschlossen` erreicht diesen Handler
/// **nie**, es dispatcht auf den pro-Fenster-Handler. Liefe diese Query je geschlossenem Fenster,
/// sähe sie nie mehr als das eine Fenster, das gerade fertig wurde.
pub const CONSUMED_HISTORY_MESSAGE_TYPE: &str = "HistoryComplete";

/// Das interne Kommando am Slice-Eingang der fenster-übergreifenden Reichweite: es trägt die
/// **Lauf-Spanne**, über die die Query gescopt wird.
///
/// Die Spanne ist hier kein Beiwerk, sondern der ganze Inhalt der Nachricht: sie grenzt **diesen**
/// Lauf gegen frühere ab. Ohne sie zöge die Query auch die Aggregate früherer Läufe herein — ein
/// **stiller** Fehler, der ein Ergebnis lieferte, nur das falsche.
pub struct EvaluateHistoryCommand {
    span: RunSpan,
}

impl EvaluateHistoryCommand {
    /// Die Lauf-Spanne. Unfehlbar: der Decoder hat eine unlesbare Spanne schon abgewiesen, was hier
    /// ankommt, ist eine echte.
    pub(crate) fn span(&self) -> RunSpan {
        self.span
    }
}

impl Command for EvaluateHistoryCommand {}

/// Der slice-gelieferte [`CommandDecoder`] (reine ACL, Shape Y): `io` extrahiert die Envelope und
/// reicht nur die nackten Strings herein — der Slice nennt **keinen** Outbox-Typ.
///
/// Die Lauf-Spanne reist im `partition_key` in der Form **`"{from}:{to}"`** — genau so, wie
/// `fenstern`s `HistoryCompleteMessage` sie setzt; beide Grenzen sind Tag-Stamps (ADR-0013,
/// Entscheidung C) und zählen einschließlich. Der `content` ist leer, **kein serde**.
///
/// Ein `partition_key`, der **keine lesbare Spanne** trägt (fehlender Trenner, nicht-numerische
/// Grenze), ist keine gültige Envelope → `None`, das `io` auf der Envelope-Ebene behandelt (nicht
/// als Handler-Fehler durchgereicht). **Parse, don't validate:** was hier durchkommt, ist eine
/// echte, wohlgeordnete [`RunSpan`] — der Handler bekommt nie eine Spanne, die er noch prüfen
/// müsste.
pub struct EvaluateHistoryCommandDecoder;

impl CommandDecoder for EvaluateHistoryCommandDecoder {
    type Command = EvaluateHistoryCommand;

    fn decode(partition_key: &str, _content: &str) -> Option<Self::Command> {
        let (from, to) = partition_key.trim().split_once(':')?;
        // `RunSpan::spanning` ordnet die Eckpunkte selbst — eine verdrehte Spanne ist damit
        // unkonstruierbar statt „geprüft" (ADR-0005), und es gibt hier keinen Zweig dafür.
        Some(EvaluateHistoryCommand {
            span: RunSpan::spanning(
                DayStamp::new(from.trim().parse().ok()?),
                DayStamp::new(to.trim().parse().ok()?),
            ),
        })
    }
}
