//! Die Wert-Objekte des Slice. Der Klartext-Engpass [`NonEmptyString`] ist die eine validierende
//! Stelle; darauf bauen die obfuskierten Token, der [`Seed`], der [`Recipient`]-Summentyp, die rohe/
//! obfuskierte Gegenpartei-Sicht sowie die Identität [`BookingId`] und der durchgereichte
//! [`PassThrough`]. Die Aggregate selbst (`IngestedBooking`/`ObfuscatedBooking`) sind über ihre
//! Identität definierte Entities und liegen in `entities/`.

mod booking_id;
mod creditor_id;
mod known_persons;
mod non_empty_string;
mod obfuscated_counterparty;
mod pass_through;
mod raw_counterparty;
mod recipient;
mod replacement;
mod scrubbing;
mod seed;

pub use booking_id::BookingId;
pub use creditor_id::{CreditorId, is_valid as is_valid_creditor_id};
pub use known_persons::KnownPersons;
pub use non_empty_string::NonEmptyString;
pub use obfuscated_counterparty::ObfuscatedCounterparty;
pub use pass_through::PassThrough;
pub use raw_counterparty::RawCounterparty;
pub use recipient::Recipient;
pub use replacement::{AccountToken, ObfuscatedText, Pseudonym, Replacement};
pub use scrubbing::is_scrubbed;
pub use seed::Seed;
