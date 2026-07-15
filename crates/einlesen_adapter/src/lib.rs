//! einlesen-Slice — **Adapter**: umschließt die blinde Domäne ([`einlesen_domain`]) und ist
//! die **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)). Seit dem
//! Kernel-Flip ist er **restlos outbox-frei**: er bindet **nur** `application_kernel`, nie
//! `shared_kernel` — die Outbox-Mechanik (Envelope, Consumer-Rolle, `UnitOfWork`) lebt
//! vollständig in `io`.
//!
//! **Reine Pipeline (ADR-0011, Entscheidung A):** der [`ReadingHandler`] ist der
//! `application_kernel::Handler` des Slice — `ReadCommand → Domäne → Response`. `io`s generische
//! `OutboxConsumer` fährt ihn: sie extrahiert die Envelope, ruft den slice-gelieferten
//! [`ReadCommandDecoder`] (Shape Y, nur nackte Strings) und dispatcht. Ein terminaler Fehler ist
//! der **nackte Domänen-`ReadError`** (hier re-exportiert), dessen **redigierte** `Display` `io`
//! spiegelt.
//!
//! **Schichtung (die eine Aufgabe dieses Layers):** der interne `ReadHandler` hängt
//! **ausschließlich** an den von `einlesen_domain` verlangten Ports (`RawLineSource`, `Calendar`,
//! `Money`, `Store`, `EventSink`). **Darüber** mappen Port-Bridges (Modul [`adapter`]) die
//! öffentlichen, mechanischen Abstractions (Modul [`abstractions`], die io erfüllt) auf genau
//! diese internen Domänen-Ports. Alle drei I/O-Schritte (Beschaffung, Ablage, Veröffentlichung)
//! sind fehlbar und terminal.
//!
//! **Öffentliche Fläche:** die Fluent-[`ReadTestApi`], die DTOs ([`Line`], [`RawRecord`]) samt
//! adapter-eigener [`Direction`]/[`Currency`], die von `io` erfüllten Seams ([`SourceSeam`] +
//! [`SourceUnavailable`], [`CalendarSeam`] + [`CalendarDay`], [`MoneySeam`] + [`DecodedAmount`]/
//! [`Sign`], [`StoreSeam`] + [`StoreUnavailable`], [`PublishSeam`] + [`PublishUnavailable`]) und
//! der von `io` verdrahtete Slice-Vertrag ([`ReadingHandler`], [`ReadCommand`]/
//! [`ReadCommandDecoder`], das opake [`IngestedMessage`], [`CONSUMED_MESSAGE_TYPE`], der
//! re-exportierte [`ReadError`]). **Crate-privat** bleiben der Domänen-`ReadHandler`, alle Bridges
//! und die Fakes. Weder `Direction` noch `CalendarDay` der Domäne werden re-exportiert — `Line`
//! trägt ausschließlich adapter-eigene Typen.
//!
//! **Datenschutz (ADR-0008-Reframe):** `einlesen` trägt rohe Werte (IBANs, Klarname) weiter —
//! das Obfuskieren ist der nachgelagerte Slice. Es gibt hier bewusst **keinen** `Serialize`-Guard:
//! rohe Werte dürfen **lokal** existieren — sie reisen **in-process** über den Quell-Seam in die
//! Domäne (`RawRecord`) und liegen in der unter der Aggregat-ID persistierten Roh-Erfassung
//! (`Line`) —, das ist kein Egress. Die ausgehende „Eingelesen"-Nachricht trägt **nur die ID** als
//! Zeiger. Der `Serialize`-Guard wandert auf die Egress-DTOs (auswerten/report).

mod abstractions;
mod adapter;
mod handler;
mod models;
mod testapi;

pub use abstractions::{
    CalendarDay, CalendarSeam, DecodedAmount, MoneySeam, PublishSeam, PublishUnavailable, Sign,
    SourceSeam, SourceUnavailable, StoreSeam, StoreUnavailable,
};
pub use handler::ReadingHandler;
pub use models::{
    CONSUMED_MESSAGE_TYPE, Currency, Direction, IngestedMessage, Line, RawRecord, ReadCommand,
    ReadCommandDecoder,
};
pub use testapi::ReadTestApi;

/// Der **nackte Domänen-Fehler** des Slice, re-exportiert (ADR-0011, Entscheidung B): der
/// [`ReadingHandler`] gibt ihn 1:1 zurück, `io` spiegelt seine **redigierte** `Display`. Bewusst
/// re-exportiert statt adapter-lokal re-deklariert — anders als die projizierten DTO-Summentypen
/// ([`Direction`]/[`Currency`]) ist der Fehler kein Datenwert, sondern der Redaktions-Vertrag
/// selbst: seine `Display` ist der **alleinige** Eigentümer des Meldungssatzes, ein Re-Deklarat
/// wäre ein Satz-Duplikat über Crates.
pub use einlesen_domain::ReadError;
