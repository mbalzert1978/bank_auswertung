//! Die **Entities** des Slice — über ihre **Identität** definierte Aggregate: in der DB
//! abgelegt und per ID auffindbar ([`ObfuscatedBooking`] per Store-vergebener ID,
//! [`IngestedBooking`] trägt seine [`BookingId`](crate::BookingId) direkt als Feld). Sie
//! orchestrieren die Wert-Objekte des Slice; der durchgereichte [`PassThrough`](crate::PassThrough)
//! reist verbatim mit.

mod ingested_booking;
mod obfuscated_booking;

pub use ingested_booking::IngestedBooking;
pub use obfuscated_booking::ObfuscatedBooking;
