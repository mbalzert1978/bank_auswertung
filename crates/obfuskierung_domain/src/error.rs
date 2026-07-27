use std::fmt;

use crate::value_objects::BookingId;

/// Warum ein I/O-Schritt des Slice (Lesen, Ablegen, Publizieren) terminal scheitert — **redigiert:**
/// eine Variante trägt die opake [`BookingId`], einen **statischen Feldnamen** oder die
/// durchgereichte io-Diagnose, nie einen Rohwert. Die ersten beiden sind redigiert **per
/// Konstruktion**; die durchgereichte Diagnose ist es **per Vertrag** — die Redaktions-Pflicht liegt
/// beim erfüllenden Seam (`StoreUnavailable`), wie bei `fenstern`/`auswerten`. Die Domäne ist
/// dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein `thiserror`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// Zur ID liegt keine Zeile im einlesen-Speicher vor. Trägt die opake [`BookingId`] (kein Rohwert).
    NotFound(BookingId),
    /// Eine vorhandene Zeile hat ein leeres/fehlendes Pflichtfeld — trägt nur den statischen Feldnamen.
    MissingMandatoryField { field: &'static str },
    /// Die Ablage der obfuskierten Buchung ist gescheitert. Trägt die opake Quell-[`BookingId`] **und**
    /// die durchgereichte io-Diagnose (`reason` — Zugriffs-Mechanik, nie ein Rohwert), formgleich mit
    /// `fenstern`/`auswerten`: ein unerreichbarer Store bleibt damit diagnostizierbar, statt dass die
    /// Diagnose an der Adapter-Grenze verfällt.
    StorageUnavailable { source: BookingId, reason: String },
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
            SourceError::StorageUnavailable { source, reason } => {
                write!(
                    f,
                    "could not store obfuscated booking for source booking {}: {reason}",
                    source.as_str()
                )
            }
        }
    }
}

impl std::error::Error for SourceError {}
