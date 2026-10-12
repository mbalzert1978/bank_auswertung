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

/// Das interne Kommando am Slice-Eingang der fenster-übergreifenden Reichweite: es trägt den
/// **Lauf** und seine **Lauf-Spanne**, über die die Query gescopt wird.
///
/// Der Lauf ist hier kein Beiwerk, sondern der Scope
/// ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)): jeder Lauf
/// paginiert dieselbe Historie, also grenzt die Spanne allein **diesen** Lauf nicht gegen frühere
/// ab — die Query zöge deren Fenster still mit herein, ein Ergebnis, nur das falsche. Den Lauf
/// braucht nur die Beschaffung (die Lese-Bridge legt ihn als Scope an den Seam); die Domäne bleibt
/// lauf-blind und sieht nur die Spanne.
pub struct EvaluateHistoryCommand {
    run: RunScope,
    span: RunSpan,
}

/// Der **Lauf**, auf den die fenster-übergreifende Query gescopt wird — adapter-eigen, weil nur die
/// Beschaffung ihn braucht (die Domäne bleibt lauf-blind, ADR-0016). Ein Newtype statt `String`:
/// **parse, don't validate** — gebaut wird er allein im Decoder aus einem nicht-leeren, getrimmten
/// Schlüsselteil; wer ihn in der Hand hält, hält einen echten Lauf. Opak: er wird nur an den Seam
/// durchgereicht, nie gedeutet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunScope(String);

impl RunScope {
    /// Parst den Lauf-Teil eines Schlüssels: getrimmt; leer → `None`.
    fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_string()))
    }

    /// Der Lauf als Zeichenkette — für den Seam, der ihn als `WHERE`-Parameter braucht.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl EvaluateHistoryCommand {
    /// Die Lauf-Spanne. Unfehlbar: der Decoder hat eine unlesbare Spanne schon abgewiesen, was hier
    /// ankommt, ist eine echte.
    pub(crate) fn span(&self) -> RunSpan {
        self.span
    }

    /// Der Lauf, auf den die Query gescopt wird — nie leer (der Decoder weist einen Schlüssel ohne
    /// Lauf ab).
    pub(crate) fn run(&self) -> &RunScope {
        &self.run
    }
}

impl Command for EvaluateHistoryCommand {}

/// Der slice-gelieferte [`CommandDecoder`] (reine ACL, Shape Y): `io` extrahiert die Envelope und
/// reicht nur die nackten Strings herein — der Slice nennt **keinen** Outbox-Typ.
///
/// Spanne und Lauf reisen im `partition_key` in der Form **`"{from}:{to}:{run}"`** — genau so, wie
/// `fenstern`s `HistoryCompleteMessage` sie setzt; beide Grenzen sind Tag-Stamps (ADR-0013,
/// Entscheidung C) und zählen einschließlich, der Lauf steht zuletzt, weil er opak ist (ADR-0016).
/// Der `content` ist leer, **kein serde**.
///
/// Ein `partition_key`, der **keine lesbare Spanne** oder **keinen Lauf** trägt (fehlender Trenner,
/// nicht-numerische Grenze, leerer Lauf), ist keine gültige Envelope → `None`, das `io` auf der Envelope-Ebene behandelt (nicht
/// als Handler-Fehler durchgereicht). **Parse, don't validate:** was hier durchkommt, ist eine
/// echte, wohlgeordnete [`RunSpan`] — der Handler bekommt nie eine Spanne, die er noch prüfen
/// müsste.
pub struct EvaluateHistoryCommandDecoder;

impl CommandDecoder for EvaluateHistoryCommandDecoder {
    type Command = EvaluateHistoryCommand;

    fn decode(partition_key: &str, _content: &str) -> Option<Self::Command> {
        let mut parts = partition_key.trim().splitn(3, ':');
        let from = parts.next()?.trim().parse().ok()?;
        let to = parts.next()?.trim().parse().ok()?;
        let run = RunScope::parse(parts.next()?)?;
        // `RunSpan::spanning` ordnet die Eckpunkte selbst — eine verdrehte Spanne ist damit
        // unkonstruierbar statt „geprüft" (ADR-0005), und es gibt hier keinen Zweig dafür.
        Some(EvaluateHistoryCommand {
            run,
            span: RunSpan::spanning(DayStamp::new(from), DayStamp::new(to)),
        })
    }
}
