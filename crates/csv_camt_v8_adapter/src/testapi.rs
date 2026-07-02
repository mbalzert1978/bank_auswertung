use std::str::FromStr;

use eigene_konten_adapter::{
    Iban as KontenIban, KonfigError, Konfigquelle, Kontenregister, KontoEintrag,
    Rolle as KontenRolle,
};

use crate::adapter::CsvCamtV8;
use crate::buchung::Buchung;
use crate::fehler::ImportError;
use crate::port::{Umsatzquelle, Zeilenquelle, ZeilenquelleError};
use crate::zeile::Umsatzzeile;

/// Fluent-TestApi des CSV-CAMT-v8-Slice ([ADR-0002](../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// arrangiert eigene Konten (rohe Strings, wie in einer Config) und bereits
/// geparste [`Umsatzzeile`]-DTOs (wie IO sie aus dem CSV bauen würde), fährt dann
/// den **echten** Adapter hinter Fake-Seams und liefert die obfuskierten
/// `Buchung`-DTOs — alles über die public API, ohne Interna zu berühren.
#[derive(Debug, Default)]
pub struct CsvImportTestApi {
    konten: Vec<(String, String)>,
    zeilen: Vec<Umsatzzeile>,
}

impl CsvImportTestApi {
    /// Neue, leere Arrange-Stufe.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fügt ein eigenes Konto hinzu (IBAN + Rolle als Rohtext — validiert beim
    /// Laden über den Eigene-Konten-Slice, wie bei einer echten Config).
    pub fn mit_konto(mut self, iban: &str, rolle: &str) -> Self {
        self.konten.push((iban.to_string(), rolle.to_string()));
        self
    }

    /// Fügt eine bereits geparste Umsatzzeile hinzu (spielt IO, das CSV parst).
    pub fn mit_zeile(mut self, zeile: Umsatzzeile) -> Self {
        self.zeilen.push(zeile);
        self
    }

    /// Führt den echten Import über die Fake-Seams aus und liefert die Buchungen —
    /// oder den `ImportError`. Eine ungültige **Test-Konten-Config** ist ein
    /// Arrange-Fehler und bricht laut ab (die Konten-Fehlerpfade prüft der
    /// Eigene-Konten-Slice selbst).
    pub fn importiere(self) -> Result<Vec<Buchung>, ImportError> {
        let adapter = CsvCamtV8::new(
            FakeZeilenquelle {
                zeilen: self.zeilen,
            },
            lade_konten(self.konten),
        );
        adapter.buchungen()
    }
}

/// Baut das Kontenregister aus den arrangierten Rohtexten. Kein `Result`: eine
/// ungültige Test-Konten-Config ist ein Arrange-Fehler und darf laut scheitern
/// (die Konten-Fehlerpfade prüft der Eigene-Konten-Slice selbst).
fn lade_konten(eintraege: Vec<(String, String)>) -> Kontenregister {
    Kontenregister::laden(&FakeKonfigquelle { eintraege }).expect("gültige Test-Konten-Config lädt")
}

/// Fake-Seam: reicht die arrangierten Zeilen durch (spielt IO, das das CSV geparst
/// hat).
struct FakeZeilenquelle {
    zeilen: Vec<Umsatzzeile>,
}

impl Zeilenquelle for FakeZeilenquelle {
    fn zeilen(&self) -> Result<Vec<Umsatzzeile>, ZeilenquelleError> {
        Ok(self.zeilen.clone())
    }
}

/// Fake-Seam für den Eigene-Konten-Slice: baut dessen Konto-DTOs aus den
/// arrangierten Rohtexten (spielt IO). Eine ungültige IBAN/Rolle wird — wie bei
/// echtem IO — zum Herkunftsfehler.
struct FakeKonfigquelle {
    eintraege: Vec<(String, String)>,
}

impl Konfigquelle for FakeKonfigquelle {
    fn eintraege(&self) -> Result<Vec<KontoEintrag>, KonfigError> {
        self.eintraege
            .iter()
            .map(|(iban, rolle)| {
                Ok(KontoEintrag {
                    iban: KontenIban::new(iban)?,
                    rolle: KontenRolle::from_str(rolle)?,
                })
            })
            .collect()
    }
}
