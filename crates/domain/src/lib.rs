//! Bank-Auswertung – Domänen-Crate.
//!
//! Rein, kein I/O: keine Datei-, Netzwerk- oder Datenbankzugriffe. Öffentliche
//! Grenze sind Value Objects, Summentypen und die schmale `KategorieAnfrage`-
//! Projektion (Anti-Corruption-Layer, siehe ADR-0002).

mod betrag;
mod buchung;
mod buchungsart;
#[path = "empfänger.rs"]
mod empfänger;
mod fingerabdruck;
mod gegenkonto;
#[path = "gläubiger_id.rs"]
mod gläubiger_id;
mod kategorie_anfrage;
mod konto_id;
mod non_empty_string;
mod obfuskierter_text;
mod pseudonym;
mod richtung;
mod strukturelle_identifikatoren;

pub use betrag::{Betrag, Währung};
pub use buchung::Buchung;
pub use buchungsart::Buchungsart;
pub use empfänger::Empfänger;
pub use fingerabdruck::Fingerabdruck;
pub use gegenkonto::Gegenkonto;
pub use gläubiger_id::GläubigerId;
pub use kategorie_anfrage::KategorieAnfrage;
pub use konto_id::KontoId;
pub use non_empty_string::NonEmptyString;
pub use obfuskierter_text::ObfuskierterText;
pub use pseudonym::Pseudonym;
pub use richtung::Richtung;
