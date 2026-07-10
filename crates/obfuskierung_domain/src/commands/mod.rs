//! Das domänen-interne Kommando des Slice: [`ObfuscateBooking`] trägt nur die **ID** der Buchung.
//! Der Adapter baut es aus dem Wire-Zeiger (ACL); die Domäne kennt keine Wire-Form.

mod obfuscate_booking;

pub use obfuscate_booking::ObfuscateBooking;
