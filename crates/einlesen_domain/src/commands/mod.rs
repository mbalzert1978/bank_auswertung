//! Das domänen-interne Kommando des Slice und sein Nutzlast-Wert-Objekt: `ReadTransactions`
//! trägt den ganzen Batch als `RawLine`-Werte. Der Adapter baut beide aus dem Wire-DTO (ACL);
//! die Domäne selbst kennt keine Wire-/serde-Form.

mod raw_line;
mod read_transactions;

pub use raw_line::RawLine;
pub use read_transactions::ReadTransactions;
