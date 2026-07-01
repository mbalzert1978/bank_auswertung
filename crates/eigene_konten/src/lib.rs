//! Eigene-Konten-Config (Baureihenfolge-Schritt 2, adapter-seitig).
//!
//! Je eigenem Konto seine IBAN + Konto-Rolle. Das ist die **eine Quelle** für
//! zwei spätere Verwendungen: die Auflösung `Gegenkonto::Eigen|Fremd` beim
//! CSV-Einlesen und die Rollen-Deutung der Analyse (Cashflow je Konto). Damit
//! beide nicht auseinanderlaufen, beantwortet **dieselbe** Instanz beide Fragen
//! über [`EigeneKonten::lookup`] → [`Kontozuordnung`].
//!
//! Die Herkunft der Config steckt hinter dem Port [`ConfigSource`] — dieser
//! Crate hängt **nicht** an `std::fs`; der konkrete Datei-Zugriff lebt außerhalb
//! (Composition Root / eigener Adapter), Dependency Inversion (ADR-0001). Der
//! reine Kern [`EigeneKonten::from_toml`] parst einen bereits gelesenen String.
//! Die IBAN ist hier reine Adapter-Konvention, **nicht** der Domänen-`KontoId`.
//!
//! Datenschutz: eine eigene IBAN ist ein struktureller Identifikator und darf
//! die Maschine nie verlassen ([`Iban`] leitet **kein** `Serialize` ab).
//!
//! # Config-Format (TOML)
//!
//! ```toml
//! [[konto]]
//! iban  = "DE00 0000 0000 0000 0000 00"   # Leerzeichen/Groß-Klein egal
//! rolle = "Giro"                          # Giro | Haushalt | Spar | Vorlauf
//! ```
//!
//! Eine ungültige IBAN, eine unbekannte Rolle, eine doppelt eingetragene IBAN
//! oder eine Config ganz ohne Konto-Eintrag führen zum Abbruch — kein
//! stillschweigendes Überspringen.

mod config_source;
mod iban;
mod kontozuordnung;
mod register;
mod rolle;

pub use config_source::{ConfigSource, ConfigSourceError};
pub use iban::{Iban, IbanError};
pub use kontozuordnung::Kontozuordnung;
pub use register::{EigeneKonten, LoadError};
pub use rolle::Rolle;
