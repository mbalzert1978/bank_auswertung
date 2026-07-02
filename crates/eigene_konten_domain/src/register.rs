use std::collections::HashMap;

use crate::iban::IbanEntity;
use crate::rolle::RolleEntity;

/// Ergebnis der Nachschlage-Frage „ist diese IBAN eine eigene?". Trägt den
/// Eigen/Fremd- **und** den Rollen-Aspekt in **einem** Typ: „eigen ohne Rolle"
/// und „fremd mit Rolle" sind damit unkonstruierbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kontozuordnung {
    /// Ein eigenes Konto — mit seiner Rolle.
    Eigen(RolleEntity),
    /// Ein fremdes Konto.
    Fremd,
}

/// Das Aggregat des Eigene-Konten-Slice: je eigenem Konto seine IBAN und Rolle,
/// die **eine Quelle** für die Eigen/Fremd-Auflösung. Rich domain — der einzige
/// Weg hinein ist [`aus_eintraegen`](Self::aus_eintraegen), das die Invarianten
/// (keine doppelte IBAN, mindestens ein Konto) erzwingt; die Frage beantwortet
/// [`zuordnung`](Self::zuordnung). Die interne Zuordnung ist gekapselt.
#[derive(Debug, Clone)]
pub struct EigeneKontenAggregat {
    konten: HashMap<IbanEntity, RolleEntity>,
}

impl EigeneKontenAggregat {
    /// Baut das Aggregat aus den validierten Einträgen. Eine doppelt
    /// eingetragene IBAN oder gar kein Eintrag ist eine Fehlkonfiguration und
    /// wird laut abgelehnt — kein stillschweigendes Überspringen, kein stilles
    /// „alles fremd".
    pub fn aus_eintraegen(
        eintraege: impl IntoIterator<Item = (IbanEntity, RolleEntity)>,
    ) -> Result<Self, KontenError> {
        let konten = eintraege
            .into_iter()
            .try_fold(HashMap::new(), |mut acc, (iban, rolle)| {
                if acc.insert(iban, rolle).is_some() {
                    Err(KontenError::DoppelteIban)
                } else {
                    Ok(acc)
                }
            })?;
        if konten.is_empty() {
            return Err(KontenError::KeineKonten);
        }
        Ok(Self { konten })
    }

    /// Beantwortet für eine IBAN beide Fragen zugleich: eigen (mit Rolle) oder
    /// fremd.
    pub fn zuordnung(&self, iban: &IbanEntity) -> Kontozuordnung {
        self.konten
            .get(iban)
            .copied()
            .map_or(Kontozuordnung::Fremd, Kontozuordnung::Eigen)
    }
}

/// Warum der Aufbau des Aggregats scheiterte. Die Meldungen nennen **keine**
/// rohe IBAN — ein struktureller Identifikator soll auch nicht über ein Log die
/// Maschine verlassen. Von Hand ausgegeben (die Domäne bindet keine
/// Fehler-Bibliothek).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KontenError {
    /// Dieselbe eigene IBAN ist mehrfach eingetragen.
    DoppelteIban,
    /// Die Config enthält gar keinen Konto-Eintrag.
    KeineKonten,
}

impl std::fmt::Display for KontenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DoppelteIban => {
                f.write_str("eine eigene IBAN ist mehrfach in der Eigene-Konten-Config eingetragen")
            }
            Self::KeineKonten => {
                f.write_str("die Eigene-Konten-Config enthält keinen Konto-Eintrag")
            }
        }
    }
}

impl std::error::Error for KontenError {}
