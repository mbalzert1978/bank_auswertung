//! CSV-CAMT-v8-Slice — **Domäne** (blind, ohne interne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
//!
//! Fachlicher Kern: das [`BuchungAggregat`] und die Wert-Typen, aus denen es sich
//! zusammensetzt. Die Domäne erhält ausschließlich bereits obfuskierte/validierte
//! Werte — rohe strukturelle Identifikatoren erreichen sie nie; das Obfuskieren
//! (Konto/Person/Freitext) und die Eigen/Fremd-Auflösung besorgt der Adapter über
//! Bridges zu den Nachbar-Slices, **bevor** er das Aggregat baut.
//!
//! Datenschutz: die Domäne bindet **kein** serde, also kann keiner ihrer Wert-
//! Typen `Serialize`/`Deserialize` ableiten. Die Werte tragen ohnehin nur
//! obfuskierte oder öffentliche Daten (`KONTO_x`, `PERSON_x`, Gläubiger-ID).

mod betrag;
mod buchung;
mod buchungsart;
#[path = "empfänger.rs"]
mod empfänger;
mod gegenkonto;
mod konto_id;
mod non_empty_string;
mod obfuskierter_text;
mod richtung;
mod rolle;

pub use betrag::{BetragEntity, WährungEntity};
pub use buchung::BuchungAggregat;
pub use buchungsart::BuchungsartEntity;
pub use empfänger::{EmpfängerEntity, GläubigerIdEntity, PseudonymEntity};
pub use gegenkonto::GegenkontoEntity;
pub use konto_id::KontoIdEntity;
pub use obfuskierter_text::ObfuskierterTextEntity;
pub use richtung::RichtungEntity;
pub use rolle::RolleEntity;
