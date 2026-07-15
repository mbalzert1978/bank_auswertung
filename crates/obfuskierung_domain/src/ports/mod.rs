//! Die von der Domäne **verlangten** Ports, an denen der Handler hängt: der Lese-Port
//! [`RawBookingSource`] (Aggregat per ID aus dem einlesen-Speicher), die Obfuskierungs-Mechanik
//! [`Obfuscator`], das Ablage-Repository [`ObfuscatedRepository`] (vergibt die neue ID) und der
//! [`BookingPublisher`] (ID-Zeiger). Lesen, Ablegen und Publizieren sind je fehlbar und terminal
//! ([`SourceError`](crate::SourceError)). Alle io-erfüllt; in den Specs tragen Fakes sie.

mod booking_publisher;
mod obfuscated_repository;
mod obfuscator;
mod raw_booking_source;

pub use booking_publisher::BookingPublisher;
pub use obfuscated_repository::ObfuscatedRepository;
pub use obfuscator::Obfuscator;
pub use raw_booking_source::RawBookingSource;
