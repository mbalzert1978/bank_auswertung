//! Obfuskierungs-Slice — **Domäne** (blind, ohne interne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
//!
//! Fachlicher Kern: die First-Seen-Vergabe stabiler Ersatz-Token für eigene
//! Konten (`KONTO_x`) und Personen (`PERSON_x`) sowie der Freitext-Engpass. Rohe
//! Identifikatoren gehen hinein, heraus kommen nur die obfuskierten Token-Werte
//! [`KontoIdEntity`]/[`PseudonymEntity`]/[`ObfuskierterTextEntity`].
//!
//! Öffentlich ist die schmale Fläche: das Aggregat [`ObfuskierungsAggregat`] mit
//! seinen Methoden, die Token-Werte, der von der Domäne **verlangte** Port
//! [`ZuordnungsSpeicher`] (für die laufübergreifende Stabilität; Persistenz
//! vertagt) samt [`Zuordnungsbestand`] und [`SpeicherError`]. Die First-Seen-
//! Register selbst sind gekapselt.
//!
//! Datenschutz: Die Domäne bindet **kein** serde, also kann kein struktureller
//! Identifikator (die Klartext-Schlüssel im [`Zuordnungsbestand`]) hier ein
//! `Serialize` ableiten; der compilergeprüfte Guard sitzt am Rand, auf dem
//! Eingangs-DTO `Iban` des Adapters.

mod aggregat;
mod non_empty_string;
mod speicher;
mod token;

pub use aggregat::{ObfuskierungsAggregat, Zuordnungsbestand};
pub use speicher::{SpeicherError, ZuordnungsSpeicher};
pub use token::{KontoIdEntity, ObfuskierterTextEntity, PseudonymEntity};
