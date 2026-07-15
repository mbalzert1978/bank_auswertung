use std::fmt;

use crate::value_objects::BookingId;

/// Warum ein I/O-Schritt des Slice (Lesen, Ablegen, Publizieren) terminal scheitert — **redigiert per
/// Konstruktion:** jede Variante trägt **nur** die opake [`BookingId`] oder einen **statischen
/// Feldnamen**, nie einen Rohwert. Die Domäne ist dependency-frei: `Display`/`Error` sind von Hand
/// geschrieben, kein `thiserror`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// Zur ID liegt keine Zeile im einlesen-Speicher vor. Trägt die opake [`BookingId`] (kein Rohwert).
    NotFound(BookingId),
    /// Eine vorhandene Zeile hat ein leeres/fehlendes Pflichtfeld — trägt nur den statischen Feldnamen.
    MissingMandatoryField { field: &'static str },
    /// Die Ablage der obfuskierten Buchung ist gescheitert. Trägt die opake Quell-[`BookingId`] (kein
    /// Rohwert); die io-seitige Diagnose bleibt am Seam.
    StorageUnavailable(BookingId),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::NotFound(id) => {
                write!(f, "referenced booking not found in source: {}", id.as_str())
            }
            SourceError::MissingMandatoryField { field } => {
                write!(f, "mandatory field missing in source booking: {field}")
            }
            SourceError::StorageUnavailable(id) => {
                write!(
                    f,
                    "could not store obfuscated booking for source booking: {}",
                    id.as_str()
                )
            }
        }
    }
}

impl std::error::Error for SourceError {}
