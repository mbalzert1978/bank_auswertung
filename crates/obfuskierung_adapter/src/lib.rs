//! Obfuskierungs-Slice — **Adapter**: umschließt die blinde Domäne ([`obfuskierung_domain`]) und ist
//! die **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)). Er ist der
//! zweite Schritt der Pipeline und die **Datenschutz-Grenze**. Seit dem Kernel-Flip ist er **restlos
//! outbox-frei**: er bindet **nur** `application_kernel` und nennt keinen Outbox-Typ — die
//! Outbox-Mechanik (Envelope, Consumer-Rolle, `UnitOfWork`) lebt vollständig in `io`.
//!
//! **Reine Pipeline (ADR-0011, Entscheidung A):** der [`ObfuscatingHandler`] ist der
//! `application_kernel::Handler` des Slice — `ObfuscateCommand → Domäne → Response`. `io`s generische
//! `OutboxConsumer` fährt ihn: sie extrahiert die Envelope, ruft den slice-gelieferten
//! [`ObfuscateCommandDecoder`] (Shape Y, nur nackte Strings; der Aggregat-Zeiger liegt im
//! `partition_key`) und dispatcht. Der Handler **liest** das rohe Aggregat per ID aus dem
//! einlesen-Speicher, obfuskiert seed-basiert, **legt die obfuskierte Buchung in der
//! `obfuskiert`-Tabelle ab** (die Persistenz vergibt die neue ID) und **publiziert genau diese neue
//! ID** als Zeiger (Tabelle = Wahrheit, Message = Pointer). Ein terminaler Fehler ist der **nackte
//! Domänen-[`SourceError`]** (hier re-exportiert), dessen **redigierte** `Display` `io` spiegelt.
//!
//! **Schichtung:** der domänen-interne [`ObfuscationHandler`](obfuskierung_domain::ObfuscationHandler)
//! hängt **nur** an den verlangten Domänen-Ports (`RawBookingSource`/`Obfuscator`/`ObfuscatedRepository`/
//! `BookingPublisher`); die Port-Bridges (Modul [`adapter`]) mappen die öffentlichen, mechanischen
//! Abstractions ([`IngestedSourceSeam`]/[`ObfuscatorSeam`]/[`ObfuscatedStoreSeam`], die io erfüllt)
//! darauf. Der Ausgang (Publisher-Bridge) zielt auf `application_kernel::Publisher`; den letzten
//! Outbox-Hop macht `io`. Betrag/Datum &c. reist als Pass-Through verbatim mit.
//!
//! **Öffentliche Fläche:** die Fluent-[`ObfuscationTestApi`], die rohe Speicher-Zeile [`IngestedRow`]
//! (die der Lese-Seam liefert) und die obfuskierten Ausgangs-DTOs ([`ObfuscatedBooking`] +
//! [`AccountId`]/[`Pseudonym`]/[`ObfuscatedText`]/[`Recipient`]), die von `io` erfüllten Seams
//! ([`IngestedSourceSeam`]/[`ObfuscatedStoreSeam`]/[`ObfuscatorSeam`]) und der von `io` verdrahtete
//! Slice-Vertrag ([`ObfuscatingHandler`], [`ObfuscateCommand`]/[`ObfuscateCommandDecoder`], das opake
//! [`ObfuscatedMessage`], [`CONSUMED_MESSAGE_TYPE`], der re-exportierte [`SourceError`]). Alles andere
//! ist crate-privat. Die eingehende Nachricht trägt **nur die ID** — kein Wire-DTO, **kein serde** im
//! Slice.
//!
//! **Datenschutz-Guard:** ab hier fließen nur Token — der `assert_not_impl_any!`-„kein
//! Serialize"-Guard liegt auf dem obfuskierten Egress-DTO [`ObfuscatedBooking`] (nicht mehr auf einer
//! rohen `Iban`): eine rohe Kennung in etwas, das die Datenschutz-Grenze verlässt, wäre ein
//! **Compile-Fehler**.

mod abstractions;
mod adapter;
mod handler;
mod models;
mod testapi;

pub use abstractions::{IngestedSourceSeam, ObfuscatedStoreSeam, ObfuscatorSeam, StoreUnavailable};
pub use handler::ObfuscatingHandler;
pub use models::{
    AccountId, CONSUMED_MESSAGE_TYPE, IngestedRow, ObfuscateCommand, ObfuscateCommandDecoder,
    ObfuscatedBooking, ObfuscatedMessage, ObfuscatedText, Pseudonym, Recipient,
};
pub use testapi::ObfuscationTestApi;

/// Der **nackte Domänen-Fehler** des Slice, re-exportiert: der [`ObfuscatingHandler`] gibt ihn 1:1
/// zurück, `io` spiegelt seine **redigierte** `Display`. Bewusst re-exportiert statt adapter-lokal
/// re-deklariert — anders als die projizierten DTO-Typen ist der Fehler kein Datenwert, sondern der
/// Redaktions-Vertrag selbst: seine `Display` ist der **alleinige** Eigentümer des Meldungssatzes, ein
/// Re-Deklarat wäre ein Satz-Duplikat über Crates.
pub use obfuskierung_domain::SourceError;

#[cfg(test)]
mod datenschutz {
    use super::ObfuscatedBooking;
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_not_impl_any;

    // Die obfuskierte Buchung überquert die Datenschutz-Grenze — sie darf **nie** serialisieren
    // (kein roher Identifikator kann so die Maschine verlassen).
    assert_not_impl_any!(ObfuscatedBooking: Serialize, Deserialize<'static>);
}
