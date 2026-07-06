//! Die Wert-Objekte des Slice — jedes parst sich **selbst** aus dem `NonEmptyString`,
//! den die Domäne an ihrer Grenze anfordert (nie ein primitives `String`/`&str`). Die
//! öffentlichen Typen werden von `lib.rs` auf Crate-Ebene gezogen.

mod betrag;
mod buchungstext;
mod datum;
mod empfaenger;
mod iban;
mod non_empty_string;
mod richtung;
mod verwendungszweck;

pub use betrag::{Betrag, Währung};
pub use buchungstext::Buchungstext;
pub use datum::{Datum, Kalendertag};
pub use empfaenger::{Empfänger, Empfängername, GläubigerId};
pub use iban::Iban;
pub use non_empty_string::NonEmptyString;
pub use richtung::Richtung;
pub use verwendungszweck::Verwendungszweck;
