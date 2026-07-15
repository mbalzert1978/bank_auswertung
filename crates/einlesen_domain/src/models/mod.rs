//! Die Request-/Config-Structs des Slice (domänenintern): das Kommando `ReadTransactions`
//! ist reiner **Auslöser** (trägt nur die `RunConfiguration`); den Batch aus `RawLine`-Werten
//! beschafft der Handler über den Port [`RawLineSource`](crate::RawLineSource). Der Adapter
//! baut das Kommando aus der Wire-Nachricht (ACL); die Domäne selbst kennt keine Wire-/serde-Form.

mod read_transactions;

pub use read_transactions::{ReadTransactions, RunConfiguration};
