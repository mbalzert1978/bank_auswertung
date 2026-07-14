use application_kernel::{Command, CommandDecoder};

/// Der Dispatch-Key der **konsumierten** Nachricht — auf ihn löst `io`s Prozessor diesen Slice als
/// Consumer auf (message_type-Auflösung). Die frühere `ConsumeBridge` trug ihn; seit
/// [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md) liegt er hier am
/// Eingang des Slice, `io` verdrahtet den [`ObfuscatingHandler`](crate::ObfuscatingHandler) darunter.
pub const CONSUMED_MESSAGE_TYPE: &str = "Ingested";

/// Das interne Kommando am Slice-Eingang: es trägt **nur den Aggregat-Zeiger** (die ID der zu
/// obfuskierenden Buchung). Der [`ObfuscatingHandler`](crate::ObfuscatingHandler) baut daraus das
/// domänen-interne `ObfuscateBooking` und liest das rohe Aggregat per ID aus dem einlesen-Speicher.
/// Es erfüllt `application_kernel::Command` (Marker).
pub struct ObfuscateCommand {
    id: String,
}

impl ObfuscateCommand {
    /// Der Aggregat-Zeiger, den der Handler in eine `BookingId` hüllt.
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
}

impl Command for ObfuscateCommand {}

/// Der slice-gelieferte [`CommandDecoder`] (reine ACL, Shape Y): `io` extrahiert die Envelope und
/// reicht nur die nackten Strings herein — der Slice nennt **keinen** Outbox-Typ. Der Aggregat-Zeiger
/// liegt im `partition_key` (Konvention: Aggregat-Identität = `partition_key`); der `content` ist
/// leer (reiner Zeiger, kein serde). Ein **leerer/blanker** `partition_key` ist kein gültiger Zeiger
/// → `None`, das `io` auf der Envelope-Ebene zu einem `Critical`-Dead-Letter macht (nicht als
/// Handler-Fehler durchgereicht). **Parse, don't validate:** die Prüfung und der übernommene Wert
/// stimmen überein — der getrimmte Zeiger wandert ins Kommando, sodass ein umrandeter `partition_key`
/// nicht als sauberer Wert geprüft, aber roh gespeichert wird (was den Lookup still verfehlen ließe).
pub struct ObfuscateCommandDecoder;

impl CommandDecoder for ObfuscateCommandDecoder {
    type Command = ObfuscateCommand;

    fn decode(partition_key: &str, _content: &str) -> Option<Self::Command> {
        let pointer = partition_key.trim();
        (!pointer.is_empty()).then(|| ObfuscateCommand {
            id: pointer.to_string(),
        })
    }
}
