//! CSV-CAMT-v8-Slice — **Adapter**: umschließt die Domäne ([`csv_camt_v8_domain`])
//! mit einem Interface und ist die einzige öffentliche Fläche des Slice
//! ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
//!
//! Nach außen spricht der Slice ausschließlich **UBQL-DTOs** ([`Buchung`] als
//! Ausgang, dazu [`Betrag`]/[`Empfänger`]/… ) — nie einen Domänentyp. Der public
//! Seam [`Zeilenquelle`] wird von **IO** erfüllt (Datei + CSV-Parse → [`Umsatzzeile`]);
//! der Adapter mappt jede Zeile über Bridges zu [`obfuskierung_adapter`] und
//! [`eigene_konten_adapter`] **obfuskierend** auf das Domänen-Aggregat und liefert
//! die Buchungen über den [`Umsatzquelle`]-Port. Die [`CsvImportTestApi`] fährt den
//! Slice von außen als Black-Box.
//!
//! # Obfuskierung am Adapter (Datenschutz-Invariante)
//!
//! Rohe strukturelle Identifikatoren (eigene IBAN, Gegen-IBAN, BIC) bleiben in der
//! [`Umsatzzeile`] und werden **nie** in eine [`Buchung`] eingebettet: Auftragskonto
//! → obfuskierte [`KontoId`] `KONTO_x`; Gegen-IBAN → [`Gegenkonto::Eigen`] (mit
//! Rolle) oder `Fremd`; Empfänger → [`Empfänger::Firma`] (Gläubiger-ID) oder
//! **fail-safe** [`Empfänger::Person`] (`PERSON_x`); Verwendungszweck →
//! [`ObfuskierterText`]. Der Adapter sichert **eine** Währung über die Datei zu.
//!
//! # Was IO übernimmt
//!
//! Das byteweise CSV-Parsen (Delimiter `;`, Quoting, CAMT-v8-Kopfzeile,
//! Zeichensatz) und die Persistenz der Obfuskierungs-Zuordnung liegen im vertagten
//! `io`-Crate; dieser Adapter bindet daher weder `csv` noch `std::fs`.

mod adapter;
mod betrag;
mod bridge;
mod buchung;
mod buchungsart;
#[path = "empfänger.rs"]
mod empfänger;
mod fehler;
mod gegenkonto;
mod konto_id;
mod obfuskierter_text;
mod port;
mod richtung;
mod rolle;
mod testapi;
mod zeile;

pub use adapter::CsvCamtV8;
pub use betrag::{Betrag, Währung};
pub use buchung::Buchung;
pub use buchungsart::Buchungsart;
pub use empfänger::{Empfänger, GläubigerId, Pseudonym};
pub use fehler::ImportError;
pub use gegenkonto::Gegenkonto;
pub use konto_id::KontoId;
pub use obfuskierter_text::ObfuskierterText;
pub use port::{Umsatzquelle, Zeilenquelle, ZeilenquelleError};
pub use richtung::Richtung;
pub use rolle::Rolle;
pub use testapi::CsvImportTestApi;
pub use zeile::Umsatzzeile;

#[cfg(test)]
mod datenschutz {
    use super::Umsatzzeile;
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_not_impl_any;

    // Die rohe Umsatzzeile trägt rohe IBANs und darf die Maschine nie verlassen —
    // kein `Serialize`/`Deserialize` (die CSV-Herkunft baut sie IO-seitig, ohne
    // dass die DTO-Form selbst serialisierbar sein muss).
    assert_not_impl_any!(Umsatzzeile: Serialize, Deserialize<'static>);
}
