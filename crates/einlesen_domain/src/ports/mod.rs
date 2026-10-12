//! Die von der Domäne **verlangten** Ports — Traits, deren Implementierung außerhalb
//! (in `io`, in Tests als Fake) liegt, wo die externen Abhängigkeiten sitzen. Ihre Verträge
//! sind wert-typisiert; die Domäne fordert nie ein primitives `String`/`&str` an.
//! [`RawLineSource`] **beschafft** die Rohdaten (der Batch reist seit
//! [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
//! nicht mehr im Kommando), `Calendar`/`Money` fragen, `Store`/`EventSink` stagen das
//! Ergebnis — den Commit besorgt io.

mod calendar;
mod currency;
mod event_sink;
mod money;
mod raw_line_source;
mod store;

pub use calendar::Calendar;
pub use currency::Currencies;
pub use event_sink::EventSink;
pub use money::Money;
pub use raw_line_source::RawLineSource;
pub use store::{Recording, Store};
