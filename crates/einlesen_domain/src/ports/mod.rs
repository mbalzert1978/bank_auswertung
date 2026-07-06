//! Die von der Domäne **verlangten** Ports — Traits, deren Implementierung außerhalb
//! (in `io`) liegt, wo die externen Abhängigkeiten sitzen. Ihre Verträge sind
//! wert-typisiert; die Domäne fordert nie ein primitives `String`/`&str` an.

mod kalender;
mod rohzeile;

pub use kalender::Kalender;
pub use rohzeile::Rohzeile;
