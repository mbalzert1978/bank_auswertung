//! Die adapter-lokalen Wire-DTOs des Slice: eingehend [`ReadTransactionsMessage`]
//! (Wire → Domäne) und ausgehend [`IngestedMessage`] (Domäne → Wire). Beide tragen die
//! serde-/Dispatch-Kenntnis, die die serde-freie Domäne bewusst nicht kennt.

pub(crate) mod ingested_message;
pub(crate) mod read_transactions_message;

pub(crate) use ingested_message::IngestedMessage;
pub(crate) use read_transactions_message::ReadTransactionsMessage;
