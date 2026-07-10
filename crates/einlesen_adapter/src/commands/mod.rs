//! Die adapter-lokalen Wire-DTOs des Slice: eingehend [`ReadTransactionsMessage`]
//! (Wire → Domäne, **serde** — deserialisiert den Batch) und ausgehend [`IngestedMessage`]
//! (Domäne → Wire, ein reiner **ID-Zeiger** ohne serde — die Nutzlast liegt im Speicher). Beide
//! tragen die Dispatch-Kenntnis, die die serde-freie Domäne bewusst nicht kennt.

pub(crate) mod ingested_message;
pub(crate) mod read_transactions_message;

pub(crate) use ingested_message::IngestedMessage;
pub(crate) use read_transactions_message::ReadTransactionsMessage;
