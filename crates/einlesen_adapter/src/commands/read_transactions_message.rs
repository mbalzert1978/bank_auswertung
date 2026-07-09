use serde::{Deserialize, Serialize};

use crate::dto::RawRecord;

/// Der Dispatch-Key, auf den der Outbox-Prozessor die Consume-Bridge dieses Slice auflöst
/// (Envelope-`message_type`). Adapter-/io-Sache — das Domänen-Kommando `ReadTransactions` (in
/// `einlesen_domain`) kennt keine Wire-Form und keinen Dispatch-Key.
pub(crate) const MESSAGE_TYPE: &str = "ReadTransactions";

/// Das serialisierbare **Wire-DTO** der `ReadTransactions`-Nachricht: die flache, serde-fähige
/// Form, die io in die Outbox schreibt und die die Consume-Bridge wieder liest
/// ([`outbox_payload`](shared_kernel::outbox_payload): camelCase). Es ist der reine
/// **Deserialisierungs-Zieltyp** — die Consume-Bridge setzt jede `RawRecord`-Zelle über
/// [`RawRecord::to_raw_line`](crate::dto::RawRecord) auf das interne Domänen-Kommando um, das der
/// Handler consumiert; so bindet der Handler **nicht** ans deserialisierte DTO. Symmetrisch zum
/// ausgehenden [`IngestedMessage`](crate::commands::IngestedMessage) (dort Domäne → Wire, hier
/// Wire → Domäne). `source` ist Envelope-/Routing-Kontext (Partition-Key der Nachricht), den der
/// Handler nicht braucht — nur der Batch `lines` wird ins Kommando übernommen.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadTransactionsMessage {
    pub(crate) source: String,
    pub(crate) lines: Vec<RawRecord>,
}
