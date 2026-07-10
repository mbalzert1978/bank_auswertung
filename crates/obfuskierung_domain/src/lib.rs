//! Obfuskierungs-Slice — **Domäne** (blind, ohne interne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md) /
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
//!
//! Der zweite Pipeline-Schritt und die **Datenschutz-Grenze**. Der [`ObfuscationHandler`]
//! orchestriert wie der `ReadHandler` gegen verlangte Ports: er **liest** das rohe Aggregat per ID
//! aus dem einlesen-Speicher ([`RawBookingSource`]), **obfuskiert** es seed-basiert
//! ([`Obfuscator`]), **legt** es ins Repository ([`ObfuscatedRepository`]) und **publiziert** seine
//! ID als Zeiger ([`BookingPublisher`]).
//!
//! Fachlicher Kern der Obfuskierung: die **seed-basierte** Ersetzung von Konten (`KONTO_…`) und
//! Privatpersonen (`PERSON_…`), die **Fail-safe-Deutung** Firma vs. Privatperson ([`Recipient`]) und
//! der Freitext-Engpass ([`ObfuscatedText`]). Gleicher Seed + gleiche Eingabe → gleiches Token
//! (zustandslos, laufstabil, reproduzierbar), der Seed wird mitpersistiert. Betrag/Datum &c. reist
//! als [`PassThrough`] unverändert mit.
//!
//! Datenschutz: Die Domäne bindet **kein** serde; der compilergeprüfte „kein Serialize"-Guard sitzt
//! am Rand, auf dem Egress-DTO `ObfuscatedBooking` des Adapters.

mod commands;
mod handler;
mod ports;
mod value_objects;

pub use commands::ObfuscateBooking;
pub use handler::{HandleError, ObfuscationHandler};
pub use ports::{
    BookingLookup, BookingPublisher, ObfuscatedRepository, Obfuscator, RawBookingSource,
};
pub use value_objects::{
    AccountToken, BookingId, CreditorId, IngestedBooking, NonEmptyString, ObfuscatedBooking,
    ObfuscatedCounterparty, ObfuscatedText, PassThrough, Pseudonym, RawCounterparty, Recipient,
    Replacement, Seed,
};
