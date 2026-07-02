//! Obfuskierungs-Slice — **Adapter**: umschließt die Domäne
//! ([`obfuskierung_domain`]) mit einem Interface und ist die einzige öffentliche
//! Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
//!
//! Public API [`Obfuskierer`]: rohe Eingänge (Eingangs-DTO [`Iban`], Namen,
//! Freitext) rein, obfuskierte DTOs ([`KontoId`], [`Pseudonym`],
//! [`ObfuskierterText`]) raus — nie ein Domänentyp. Andere Slices konsumieren
//! diese API über eine Bridge, **bevor** ihre Domäne rohe Identifikatoren sieht.
//! Der von der Domäne verlangte [`ZuordnungsSpeicher`]-Port ist re-exportiert
//! (echtes IO erfüllt ihn später); die [`ObfuskierungTestApi`] fährt den Slice als
//! Black-Box.
//!
//! Datenschutz: [`Iban`] ist ein struktureller Identifikator und leitet **kein**
//! `Serialize`/`Deserialize` ab (compilergeprüft unten).

mod iban;
mod obfuskierer;
mod testapi;
mod token;

pub use iban::Iban;
pub use obfuskierer::Obfuskierer;
pub use obfuskierung_domain::{SpeicherError, ZuordnungsSpeicher, Zuordnungsbestand};
pub use testapi::ObfuskierungTestApi;
pub use token::{KontoId, ObfuskierterText, Pseudonym};

#[cfg(test)]
mod datenschutz {
    use super::Iban;
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_not_impl_any;

    // Das IBAN-Eingangs-DTO darf die Maschine nicht verlassen.
    assert_not_impl_any!(Iban: Serialize, Deserialize<'static>);
}
