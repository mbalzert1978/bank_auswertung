//! Eigene-Konten-Slice — **Adapter**: umschließt die Domäne
//! ([`eigene_konten_domain`]) mit einem Interface und ist die einzige
//! öffentliche Fläche des Slice
//! ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
//!
//! Nach außen spricht der Slice ausschließlich **UBQL-DTOs** ([`Iban`], [`Rolle`],
//! [`Zuordnung`], [`KontoEintrag`]) — nie einen Domänentyp. Der public Seam
//! [`Konfigquelle`] wird von **IO** erfüllt (Datei + TOML → DTOs); die Facade
//! [`Kontenregister`] baut daraus das Domänen-Aggregat und beantwortet die
//! Zuordnung. Die [`KontenTestApi`] fährt den Slice von außen als Black-Box.
//!
//! Datenschutz: [`Iban`] ist ein struktureller Identifikator und leitet **kein**
//! `Serialize`/`Deserialize` ab (compilergeprüft unten).

mod iban;
mod konfigquelle;
mod register;
mod rolle;
mod testapi;

pub use eigene_konten_domain::IbanError;
pub use iban::Iban;
pub use konfigquelle::{KonfigError, Konfigquelle, KontoEintrag};
pub use register::{Kontenregister, LadeError, Zuordnung};
pub use rolle::{Rolle, RolleError};
pub use testapi::{GeladenesRegister, KontenTestApi};

#[cfg(test)]
mod datenschutz {
    use super::Iban;
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_not_impl_any;

    // Auch das IBAN-DTO am Rand darf die Maschine nicht verlassen.
    assert_not_impl_any!(Iban: Serialize, Deserialize<'static>);
}
