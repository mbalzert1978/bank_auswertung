//! Das domänen-interne Kommando des Slice und das Nutzlast-Wert-Objekt der Beschaffung:
//! `ReadTransactions` ist reiner **Auslöser** (trägt nur die `RunConfiguration`); den
//! Batch aus `RawLine`-Werten beschafft der Handler über den Port
//! [`RawLineSource`](crate::RawLineSource). Der Adapter baut das Kommando aus der
//! Wire-Nachricht (ACL); die Domäne selbst kennt keine Wire-/serde-Form.

mod raw_line;
mod read_transactions;

pub use raw_line::RawLine;
pub use read_transactions::{ReadTransactions, RunConfiguration};
