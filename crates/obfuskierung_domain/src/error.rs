use std::fmt;

use crate::value_objects::BookingId;

/// Warum ein I/O-Schritt des Slice (Lesen, Ablegen, Publizieren) terminal scheitert — **redigiert:**
/// eine Variante trägt die opake [`BookingId`], einen **statischen Feldnamen** oder die
/// durchgereichte io-Diagnose, nie einen Rohwert. Die ersten beiden sind redigiert **per
/// Konstruktion**; die durchgereichte Diagnose ist es **per Vertrag** — die Redaktions-Pflicht liegt
/// beim erfüllenden Seam (`SourceUnavailable`/`StoreUnavailable`), wie bei `fenstern`/`auswerten`.
/// Die Domäne ist dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein `thiserror`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObfuscationError {
    /// Zur ID liegt keine Zeile im einlesen-Speicher vor. Trägt die opake [`BookingId`] (kein Rohwert).
    NotFound(BookingId),
    /// Die **Beschaffung** der Zeile ist gescheitert (unerreichbarer Lese-Speicher, kaputtes Schema) —
    /// etwas kategorisch anderes als [`NotFound`](Self::NotFound), wo der Speicher antwortete und
    /// nichts vorlag. Trägt **nur** die durchgereichte io-Diagnose (`reason` — Beschaffungs-Mechanik,
    /// nie ein Rohwert) und ist damit formgleich mit `fenstern`/`auswerten`: ein unerreichbarer
    /// Lese-Speicher bleibt diagnostizierbar, statt als „nicht gefunden" zu verschwinden.
    SourceUnavailable { reason: String },
    /// Eine vorhandene Zeile hat ein leeres/fehlendes Pflichtfeld — trägt nur den statischen Feldnamen.
    MissingMandatoryField { field: &'static str },
    /// Die Ablage der obfuskierten Buchung ist gescheitert. Trägt **nur** die durchgereichte
    /// io-Diagnose (`reason` — Zugriffs-Mechanik, nie ein Rohwert) und ist damit formgleich mit
    /// `fenstern`/`auswerten`: ein unerreichbarer Store bleibt diagnostizierbar, statt dass die
    /// Diagnose an der Adapter-Grenze verfällt. Die opake ID gehört in
    /// [`NotFound`](Self::NotFound), wo sie die eigentliche Aussage ist — hier trüge sie nur, was
    /// ohnehin im `partition_key` der Nachricht steht.
    StorageUnavailable { reason: String },
}

impl fmt::Display for ObfuscationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObfuscationError::NotFound(id) => {
                write!(f, "referenced booking not found in source: {}", id.as_str())
            }
            ObfuscationError::SourceUnavailable { reason } => {
                write!(f, "could not read source booking: {reason}")
            }
            ObfuscationError::MissingMandatoryField { field } => {
                write!(f, "mandatory field missing in source booking: {field}")
            }
            ObfuscationError::StorageUnavailable { reason } => {
                write!(f, "could not store obfuscated booking: {reason}")
            }
        }
    }
}

impl std::error::Error for ObfuscationError {}
