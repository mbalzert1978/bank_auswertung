//! Die Wert-Objekte des Slice — jedes parst sich **selbst** aus dem `NonEmptyString`,
//! den die Domäne an ihrer Grenze anfordert (nie ein primitives `String`/`&str`). Die
//! öffentlichen Typen werden von `lib.rs` auf Crate-Ebene gezogen.

mod amount;
mod booking_text;
mod date;
mod direction;
mod iban;
mod non_empty_string;
mod purpose;
mod recipient;

pub use amount::{Amount, Currency};
pub use booking_text::BookingText;
pub use date::{CalendarDay, Date};
pub use direction::Direction;
pub use iban::Iban;
pub use non_empty_string::NonEmptyString;
pub use purpose::Purpose;
pub use recipient::{CreditorId, Recipient, RecipientName};
