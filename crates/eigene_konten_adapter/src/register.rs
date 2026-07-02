use eigene_konten_domain::{EigeneKontenAggregat, KontenError, Kontozuordnung};

use crate::iban::Iban;
use crate::konfigquelle::{KonfigError, Konfigquelle};
use crate::rolle::Rolle;

/// Die public API-Antwort auf „ist diese IBAN eine eigene?" — das UBQL-DTO, in
/// dem der Slice nach außen (auch Slice→Slice) spricht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zuordnung {
    /// Ein eigenes Konto — mit seiner Rolle.
    Eigen(Rolle),
    /// Ein fremdes Konto.
    Fremd,
}

/// Der Adapter des Eigene-Konten-Slice: lädt die Config über den
/// [`Konfigquelle`]-Seam, baut daraus das Domänen-Aggregat und beantwortet
/// darüber die Zuordnungs-Frage nach außen. Umschließt die Domäne mit einem
/// Interface — Aufrufer sehen nur DTOs, nie einen Domänentyp.
#[derive(Debug, Clone)]
pub struct Kontenregister {
    aggregat: EigeneKontenAggregat,
}

impl Kontenregister {
    /// Lädt und validiert die Config über den Seam. Ein Herkunfts-/Formatfehler,
    /// eine doppelte IBAN oder eine Config ohne Eintrag bricht laut ab.
    pub fn laden(quelle: &impl Konfigquelle) -> Result<Self, LadeError> {
        let eintraege = quelle.eintraege().map_err(LadeError::Quelle)?;
        let domain_eintraege = eintraege
            .into_iter()
            .map(|eintrag| (eintrag.iban.entity().clone(), eintrag.rolle.entity()));
        let aggregat = EigeneKontenAggregat::aus_eintraegen(domain_eintraege)?;
        Ok(Self { aggregat })
    }

    /// Beantwortet die Zuordnung für eine IBAN — eigen (mit Rolle) oder fremd.
    pub fn zuordnung(&self, iban: &Iban) -> Zuordnung {
        match self.aggregat.zuordnung(iban.entity()) {
            Kontozuordnung::Eigen(rolle) => Zuordnung::Eigen(Rolle::from_entity(rolle)),
            Kontozuordnung::Fremd => Zuordnung::Fremd,
        }
    }
}

/// Warum das Laden der Config scheiterte. Trägt **keine** rohe IBAN (Datenschutz).
#[derive(Debug, thiserror::Error)]
pub enum LadeError {
    /// Die Config-Quelle konnte die Einträge nicht liefern (Datei-I/O, TOML,
    /// ungültige IBAN/Rolle beim Aufbau der DTOs).
    #[error("Config konnte nicht geladen werden: {0}")]
    Quelle(#[source] KonfigError),
    /// Dieselbe eigene IBAN ist mehrfach eingetragen.
    #[error("eine eigene IBAN ist mehrfach in der Eigene-Konten-Config eingetragen")]
    DoppelteIban,
    /// Die Config enthält keinen Konto-Eintrag.
    #[error("die Eigene-Konten-Config enthält keinen Konto-Eintrag")]
    KeineKonten,
}

impl From<KontenError> for LadeError {
    fn from(error: KontenError) -> Self {
        match error {
            KontenError::DoppelteIban => Self::DoppelteIban,
            KontenError::KeineKonten => Self::KeineKonten,
        }
    }
}
