use std::str::FromStr;

use eigene_konten_domain::{IbanEntity, IbanError};

/// Das UBQL-DTO für eine IBAN am Slice-Rand — der Typ, in dem der public Seam und
/// die Slice→Slice-API sprechen. Selbstvalidierend: [`Iban::new`]/`from_str`
/// prüfen über den Domänen-Konstruktor (Prüfsumme lebt einmal in
/// [`IbanEntity`]). Trägt die validierte Entity intern und leitet — wie die
/// Entity — **kein** `Serialize`/`Deserialize` ab: eine IBAN ist ein
/// struktureller Identifikator und verlässt die Maschine nie
/// ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iban(IbanEntity);

impl Iban {
    /// Baut das DTO aus rohem Text — validiert (Struktur + mod-97) durch den
    /// Domänen-Konstruktor.
    pub fn new(input: impl AsRef<str>) -> Result<Self, IbanError> {
        IbanEntity::parse(input).map(Self)
    }

    /// Die normalisierte IBAN (Großbuchstaben, ohne Leerzeichen).
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Die validierte Domänen-Entity — nur crate-intern, für das Mapping DTO →
    /// Domäne in den Bridges dieses Slice.
    pub(crate) fn entity(&self) -> &IbanEntity {
        &self.0
    }
}

impl FromStr for Iban {
    type Err = IbanError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::new(input)
    }
}
