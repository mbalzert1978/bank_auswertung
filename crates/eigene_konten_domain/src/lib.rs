//! Eigene-Konten-Slice — **Domäne** (blind, ohne interne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
//!
//! Fachlicher Kern: je eigenem Konto seine IBAN + Rolle, als die **eine Quelle**
//! für die Auflösung `eigen (mit Rolle) | fremd`. Öffentlich ist die schmale
//! Fläche — das Aggregat [`EigeneKontenAggregat`] mit seinen Methoden, die
//! Wert-Typen [`IbanEntity`]/[`RolleEntity`], das Ergebnis [`Kontozuordnung`] und
//! die Fehler. Woher die Config stammt (Datei, TOML, …) weiß dieser Crate
//! **nicht**: das besorgt der Adapter über seine public Seams; die Domäne wird
//! nur aus bereits validierten Einträgen zusammengesetzt.
//!
//! Datenschutz: [`IbanEntity`] ist ein struktureller Identifikator und leitet
//! **kein** `Serialize`/`Deserialize` ab (compilergeprüft in `iban::tests`).

mod iban;
mod register;
mod rolle;

pub use iban::{IbanEntity, IbanError};
pub use register::{EigeneKontenAggregat, KontenError, Kontozuordnung};
pub use rolle::RolleEntity;
