use serde::{Deserialize, Serialize};

/// Der Dispatch-Key, auf den der Outbox-Prozessor die Consume-Bridge dieses Slice auflöst
/// (Envelope-`message_type`). Adapter-/io-Sache — das Domänen-Kommando `ReadTransactions` (in
/// `einlesen_domain`) kennt keine Wire-Form und keinen Dispatch-Key.
pub(crate) const MESSAGE_TYPE: &str = "ReadTransactions";

/// Das serialisierbare **Wire-DTO** der `ReadTransactions`-Nachricht — seit
/// [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
/// eine **feldlose** Payload (`{}`): die Nachricht ist reiner Auslöser, den Batch beschafft
/// der Handler über den Quell-Seam, und die Quelle lebt allein im Envelope-`partition_key`
/// (ADR-0008-Envelope-Konvention — ein `source`-Payload-Feld duplizierte ihn nur). Das DTO
/// bleibt als reiner **Deserialisierungs-Zieltyp** bestehen — bewusst nicht als leerer
/// Envelope-Inhalt: den mappt die `TypedOutboxHandler`-Verdrahtung auf `OutboxError::Critical`.
/// Ein Wire-Konfigurationsfeld kommt erst, wenn die `RunConfiguration` Inhalt hat.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ReadTransactionsMessage {}
