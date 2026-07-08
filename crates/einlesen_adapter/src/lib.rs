//! einlesen-Slice — **Adapter**: umschließt die blinde Domäne ([`einlesen_domain`]) und ist
//! die **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Er ist der
//! Kopf der event-getriebenen Pipeline: eine `ReadTransactions`-Nachricht trägt den ganzen Batch
//! (alle Rohzeilen) und **fächert pro Rohzeile zu je einem „Eingelesen"-Command auf** (kein
//! Batch-Command stromabwärts). Scheitert eine gelesene Zeile am Aggregat, ist das **terminal**:
//! der Handler steigt mit dem Fehler aus, der über die Consume-Bridge zu `OutboxError::Warning`
//! bubblet — die **io-Outbox** dead-lettert die Nachricht (der Slice dead-lettert nicht selbst).
//!
//! **Schichtung (die eine Aufgabe dieses Layers):** ein interner [`ReadHandler`](einlesen_domain::ReadHandler)
//! hängt **ausschließlich** an den von `einlesen_domain` verlangten Ports (`Calendar`, `Store`,
//! `EventSink`) — nie an adapter-erfundenen Ports. **Darüber** mappen Port-Bridges (Modul
//! [`bridges`]) die öffentlichen, mechanischen Abstractions (Modul [`abstractions`], die IO
//! erfüllt) auf genau diese internen Domänen-Ports. Vorbild: `examples/Downloader`
//! (`DownloaderHandler` hängt nur an `IFilePuller`/`IDownloadLedger`; `SftpPullerAdapter` mappt
//! den öffentlichen `ISftpPuller`-Seam darauf).
//!
//! **Genau ein öffentlicher Zugang** (Direktive: eine Adapter-Grenze): die [`ReadTestApi`],
//! die DTOs ([`Line`], [`RawRecord`]) und die öffentlichen Abstractions
//! ([`CalendarSeam`] + ihr Rückgabetyp [`CalendarDay`]) — dazu das adapter-eigene [`Direction`].
//! **Alles andere ist crate-privat**: der Handler, alle Bridges, die adapter-lokalen Wire-DTOs
//! (`ReadTransactionsMessage`/`IngestedMessage`) und die Fakes — keiner dieser Typen ist von
//! außen erreichbar. Weder `Direction` noch `CalendarDay` der Domäne werden re-exportiert —
//! `Line` trägt ausschließlich adapter-eigene Typen.
//!
//! **Datenschutz (ADR-0008-Reframe):** `einlesen` trägt rohe Werte (IBANs, Klarname) weiter —
//! das Obfuskieren ist der nachgelagerte Slice. Es gibt hier bewusst **keinen**
//! `Serialize`-Guard: rohe Werte dürfen im serialisierbaren Wire-DTO durch die **lokale**
//! Outbox reisen (kein Egress); der Guard wandert auf die Egress-DTOs (auswerten/report).

mod abstractions;
mod bridges;
mod commands;
mod dto;
mod testapi;

pub use abstractions::{CalendarDay, CalendarSeam};
pub use dto::{Direction, Line, RawRecord};
pub use testapi::ReadTestApi;
