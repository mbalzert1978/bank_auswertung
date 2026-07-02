use std::str::FromStr;

use eigene_konten_domain::IbanError;

use crate::iban::Iban;
use crate::konfigquelle::{KonfigError, Konfigquelle, KontoEintrag};
use crate::register::{Kontenregister, LadeError, Zuordnung};
use crate::rolle::Rolle;

/// Fluent-TestApi des Eigene-Konten-Slice ([ADR-0002](../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// arrangiert eigene Konten über rohe Strings (wie sie in einer Config stünden),
/// lädt sie über den **echten** Adapter hinter einem Fake-Seam und beantwortet
/// die Zuordnung — alles über die public DTOs, ohne Interna zu berühren.
#[derive(Debug, Default)]
pub struct KontenTestApi {
    eintraege: Vec<(String, String)>,
}

impl KontenTestApi {
    /// Neue, leere Arrange-Stufe.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fügt ein eigenes Konto hinzu (IBAN + Rolle als Rohtext — Validierung
    /// passiert beim Laden, wie bei einer echten Config).
    pub fn mit_konto(mut self, iban: &str, rolle: &str) -> Self {
        self.eintraege.push((iban.to_string(), rolle.to_string()));
        self
    }

    /// Lädt die arrangierte Config über den echten [`Kontenregister::laden`]
    /// hinter einem Fake-Seam.
    pub fn lade(self) -> Result<GeladenesRegister, LadeError> {
        let quelle = FakeKonfigquelle {
            eintraege: self.eintraege,
        };
        Kontenregister::laden(&quelle).map(GeladenesRegister)
    }
}

/// Das geladene Register der TestApi — beantwortet die Zuordnung bequem über
/// rohen IBAN-Text.
pub struct GeladenesRegister(Kontenregister);

impl GeladenesRegister {
    /// Zuordnung für eine IBAN als Rohtext (das DTO wird intern validiert gebaut).
    pub fn zuordnung(&self, iban: &str) -> Result<Zuordnung, IbanError> {
        Ok(self.0.zuordnung(&Iban::new(iban)?))
    }
}

/// Fake-Seam: baut die DTOs aus den arrangierten Rohtexten (spielt die Rolle von
/// IO). Eine ungültige IBAN oder unbekannte Rolle wird — wie bei echtem IO — zum
/// Herkunftsfehler.
struct FakeKonfigquelle {
    eintraege: Vec<(String, String)>,
}

impl Konfigquelle for FakeKonfigquelle {
    fn eintraege(&self) -> Result<Vec<KontoEintrag>, KonfigError> {
        self.eintraege
            .iter()
            .map(|(iban, rolle)| {
                Ok(KontoEintrag {
                    iban: Iban::new(iban)?,
                    rolle: Rolle::from_str(rolle)?,
                })
            })
            .collect()
    }
}
