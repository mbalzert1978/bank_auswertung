use obfuskierung_domain::{ObfuskierungsAggregat, SpeicherError, ZuordnungsSpeicher};

use crate::iban::Iban;
use crate::token::{KontoId, ObfuskierterText, Pseudonym};

/// Der Adapter des Obfuskierungs-Slice und seine **public API**: nimmt rohe
/// Eingänge (IBAN-DTO, Namen, Freitext) und liefert ausschließlich obfuskierte
/// DTOs zurück. Umschließt das Domänen-Aggregat — Aufrufer sehen nie einen
/// Domänentyp.
///
/// Ein `Obfuskierer` wird über den [`ZuordnungsSpeicher`]-Seam geladen (er erbt
/// dessen bisherigen Bestand → laufübergreifende Stabilität) und kann seinen
/// fortgeschriebenen Bestand über [`sichern`](Self::sichern) zurückschreiben. Die
/// Persistenz dahinter ist vertagt; aktuell trägt ein In-Memory-Fake den Seam.
#[derive(Debug, Clone, Default)]
pub struct Obfuskierer {
    aggregat: ObfuskierungsAggregat,
}

impl Obfuskierer {
    /// Ein frischer Obfuskierer ohne vorherigen Bestand — für einen Lauf, der
    /// (noch) nicht an einem Store hängt. Die laufübergreifende Stabilität
    /// entsteht erst, wenn ein Store über [`laden`](Self::laden)/[`sichern`](Self::sichern)
    /// dahinterliegt; die Persistenz dafür ist vertagt.
    pub fn new() -> Self {
        Self::default()
    }

    /// Lädt den bisherigen Bestand aus dem Store und startet die Vergabe daraus.
    pub fn laden(speicher: &impl ZuordnungsSpeicher) -> Result<Self, SpeicherError> {
        let bestand = speicher.laden()?;
        Ok(Self {
            aggregat: ObfuskierungsAggregat::aus_bestand(bestand),
        })
    }

    /// Obfuskiert eine eigene IBAN zur stabilen `KONTO_x`. Gültigkeit und
    /// Eigen/Fremd sind Sache vorgelagerter Slices — hier wird nur ein Token
    /// vergeben (der `Iban`-Eingang garantiert bereits eine normalisierte,
    /// nicht-leere IBAN).
    pub fn obfuskiere_konto(&mut self, iban: &Iban) -> KontoId {
        KontoId::from_entity(self.aggregat.obfuskiere_konto(iban.as_str()))
    }

    /// Obfuskiert einen Empfänger-Namen zum stabilen `PERSON_x`. `None`, wenn der
    /// Name leer ist — dann gibt es niemanden zu pseudonymisieren.
    pub fn obfuskiere_person(&mut self, name: &str) -> Option<Pseudonym> {
        self.aggregat
            .obfuskiere_person(name)
            .map(Pseudonym::from_entity)
    }

    /// Führt Freitext durch den Obfuskierungs-Engpass. `None`, wenn nach dem
    /// Trimmen nichts bleibt.
    pub fn obfuskiere_text(&self, roh: &str) -> Option<ObfuskierterText> {
        self.aggregat
            .obfuskiere_text(roh)
            .map(ObfuskierterText::from_entity)
    }

    /// Sichert den fortgeschriebenen Vergabe-Bestand in den Store zurück — damit
    /// die Token beim nächsten Lauf stabil bleiben.
    pub fn sichern(&self, speicher: &mut impl ZuordnungsSpeicher) -> Result<(), SpeicherError> {
        speicher.sichern(&self.aggregat.bestand())
    }
}
