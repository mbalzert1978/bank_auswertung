use crate::iban::Iban;
use crate::rolle::Rolle;

/// Ein roher Konto-Eintrag der Config, als DTO (IBAN + Rolle, beide bereits
/// validiert). Genau die Form, die der [`Konfigquelle`]-Seam liefert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KontoEintrag {
    /// Die IBAN des eigenen Kontos.
    pub iban: Iban,
    /// Die Rolle dieses Kontos.
    pub rolle: Rolle,
}

/// Der public Seam für die Config-Herkunft: liefert die bereits geparsten,
/// validierten [`KontoEintrag`]-DTOs. **IO** erfüllt diesen Seam (liest die
/// Datei, parst TOML, baut die DTOs über ihre selbstvalidierenden Konstruktoren)
/// und reicht sie herunter — dieser Adapter-Crate hängt damit an keiner Datei-
/// und keiner Format-Bibliothek
/// ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
pub trait Konfigquelle {
    /// Liefert die Konto-Einträge — oder einen beliebigen Herkunfts-/Formatfehler.
    fn eintraege(&self) -> Result<Vec<KontoEintrag>, KonfigError>;
}

/// Fehler eines [`Konfigquelle`]-Seams — bewusst offen für beliebige Herkunft
/// (Datei-I/O, TOML-Parse, ungültige IBAN/Rolle beim Aufbau der DTOs, …).
pub type KonfigError = Box<dyn std::error::Error + Send + Sync>;
