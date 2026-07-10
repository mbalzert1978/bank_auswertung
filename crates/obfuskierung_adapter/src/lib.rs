//! Obfuskierungs-Slice — **Adapter**: umschließt die blinde Domäne ([`obfuskierung_domain`]) und ist
//! die **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Er ist der zweite
//! Schritt der Pipeline und die **Datenschutz-Grenze**: konsumiert einen `Ingested`-**Zeiger** (nur
//! die ID in der Outbox-Hülle), **liest** das rohe Aggregat per ID aus dem einlesen-Speicher,
//! obfuskiert seed-basiert, **legt die ganze obfuskierte Buchung in der `obfuskiert`-Tabelle ab** und
//! **publiziert nur die ID** als Zeiger (Tabelle = Wahrheit, Message = Pointer) — Ablage und Publish
//! in **einer** `UnitOfWork`.
//!
//! **Schichtung:** ein interner [`ObfuscationHandler`](obfuskierung_domain::ObfuscationHandler) hängt
//! **nur** an den verlangten Domänen-Ports (`RawBookingSource`/`Obfuscator`/`ObfuscatedRepository`/
//! `BookingPublisher`); die Port-Bridges (Modul [`bridges`]) mappen die öffentlichen, mechanischen
//! Abstractions ([`IngestedSourceSeam`]/[`ObfuscatorSeam`]/[`ObfuscatedStoreSeam`], die io erfüllt)
//! bzw. den `OutboxPublisher` darauf. Betrag/Datum &c. reist als Pass-Through verbatim mit.
//!
//! **Genau ein öffentlicher Zugang:** die [`ObfuscationTestApi`], die rohe Speicher-Zeile
//! [`IngestedRow`] (die der Lese-Seam liefert) und die obfuskierten Ausgangs-DTOs
//! ([`ObfuscatedBooking`] + [`AccountId`]/[`Pseudonym`]/[`ObfuscatedText`]/[`Recipient`]) sowie die
//! öffentlichen Abstractions. Alles andere ist crate-privat. Die eingehende Nachricht trägt **nur die
//! ID** — kein Wire-DTO, **kein serde** im Slice.
//!
//! **Datenschutz-Guard:** ab hier fließen nur Token — der `assert_not_impl_any!`-„kein
//! Serialize"-Guard liegt auf dem obfuskierten Egress-DTO [`ObfuscatedBooking`] (nicht mehr auf einer
//! rohen `Iban`): eine rohe Kennung in etwas, das die Datenschutz-Grenze verlässt, wäre ein
//! **Compile-Fehler**.

mod abstractions;
mod bridges;
mod commands;
mod dto;
mod testapi;

pub use abstractions::{IngestedSourceSeam, ObfuscatedStoreSeam, ObfuscatorSeam};
pub use dto::{AccountId, IngestedRow, ObfuscatedBooking, ObfuscatedText, Pseudonym, Recipient};
pub use testapi::ObfuscationTestApi;

#[cfg(test)]
mod datenschutz {
    use super::ObfuscatedBooking;
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_not_impl_any;

    // Die obfuskierte Buchung überquert die Datenschutz-Grenze — sie darf **nie** serialisieren
    // (kein roher Identifikator kann so die Maschine verlassen).
    assert_not_impl_any!(ObfuscatedBooking: Serialize, Deserialize<'static>);
}
