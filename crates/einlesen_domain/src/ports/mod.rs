//! Die von der Domäne **verlangten** Ports — Traits, deren Implementierung außerhalb
//! (in `io`, in Tests als Fake) liegt, wo die externen Abhängigkeiten sitzen. Ihre Verträge
//! sind wert-typisiert; die Domäne fordert nie ein primitives `String`/`&str` an. Alle
//! ausgehend: `Calendar` fragt, `Store`/`EventSink` stagen das Ergebnis — den Commit
//! besorgt io. Die Rohdaten dagegen trägt der Batch selbst als `RawLine`-Werte, kein Port.

mod calendar;
mod event_sink;
mod store;

pub use calendar::Calendar;
pub use event_sink::EventSink;
pub use store::Store;
