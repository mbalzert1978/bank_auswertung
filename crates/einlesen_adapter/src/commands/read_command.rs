use application_kernel::{Command, CommandDecoder};

/// Der Dispatch-Key der **konsumierten** Nachricht — auf ihn löst `io`s Prozessor diesen Slice
/// als Consumer auf (message_type-Auflösung). Die frühere `ConsumeBridge` trug ihn; seit
/// [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md) liegt er hier
/// am Eingang des Slice, `io` verdrahtet den `ReadingHandler` darunter.
pub const CONSUMED_MESSAGE_TYPE: &str = "ReadTransactions";

/// Das interne Kommando am Slice-Eingang — seit [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
/// ein reiner **Auslöser** ohne Nutzlast: die Konfiguration entscheidet erst der io-Schritt, den
/// Batch beschafft der Handler über den Quell-Seam. Es erfüllt `application_kernel::Command`
/// (Marker); der [`ReadingHandler`](crate::ReadingHandler) verarbeitet es.
pub struct ReadCommand;

impl Command for ReadCommand {}

/// Der slice-gelieferte [`CommandDecoder`] (reine ACL, Shape Y): `io` extrahiert die Envelope und
/// reicht nur die nackten Strings herein — der Slice nennt **keinen** Outbox-Typ. Da
/// [`ReadCommand`] ein feldloser Auslöser ist, ist der Inhalt **stets gültig** (leerer `content`
/// ist kein Fehler): der Decoder liefert immer `Some`. Die Quelle reist zwar als `partition_key`,
/// wird aber (wie bisher) verworfen — was die `RunConfiguration` trägt, entscheidet erst der
/// io-Schritt.
pub struct ReadCommandDecoder;

impl CommandDecoder for ReadCommandDecoder {
    type Command = ReadCommand;

    fn decode(_partition_key: &str, _content: &str) -> Option<Self::Command> {
        Some(ReadCommand)
    }
}
