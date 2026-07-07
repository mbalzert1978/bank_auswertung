//! einlesen-Slice — **Adapter**: umschließt die blinde Domäne ([`einlesen_domain`]) und ist
//! die **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Er ist der
//! Kopf der event-getriebenen Pipeline: eine `UmsätzeEinlesen`-Nachricht trägt den ganzen Batch
//! (alle Rohzeilen) und **fächert pro Rohzeile zu je einem `Eingelesen`-Command auf** (kein
//! Batch-Command stromabwärts). Scheitert eine gelesene Zeile am Aggregat, ist das **terminal**:
//! der Handler steigt mit dem Fehler aus, der über die Consume-Bridge zu `OutboxError::Warning`
//! bubblet — die **io-Outbox** dead-lettert die Nachricht (der Slice dead-lettert nicht selbst).
//!
//! **Schichtung (die eine Aufgabe dieses Layers):** ein interner [`EinleseHandler`] hängt
//! **ausschließlich** an den von `einlesen_domain` verlangten Ports (`Kalender`) + dem
//! injizierten `Publisher<Eingelesen>` — nie an adapter-erfundenen Ports. **Darüber** mappen
//! Port-Bridges (z. B. [`kalender_bridge::KalenderBridge`]) die öffentlichen, mechanischen
//! Abstractions (Modul [`abstractions`], die IO erfüllt) auf genau diese internen Domänen-Ports.
//! Vorbild: `examples/Downloader` (`DownloaderHandler` hängt nur an `IFilePuller`/
//! `IDownloadLedger`; `SftpPullerAdapter` mappt den öffentlichen `ISftpPuller`-Seam darauf).
//!
//! **Genau ein öffentlicher Zugang** (Direktive: eine Adapter-Grenze): die [`EinlesenTestApi`],
//! die DTOs ([`Zeile`], [`Rohsatz`]) und die öffentlichen Abstractions
//! ([`abstractions::KalenderSeam`] + ihr Rückgabetyp [`abstractions::Kalendertag`]) — dazu das
//! adapter-eigene [`zeile::Richtung`]. **Alles andere ist crate-privat**: der Handler, alle
//! Bridges (Consume-/Publisher-/Kalender-Bridge), die adapter-lokalen Kommandos
//! (`UmsätzeEinlesen`/`Eingelesen`/`EingelesenMessage`) und die Fakes — keiner dieser Typen ist
//! von außen erreichbar. Weder `Richtung` noch `Kalendertag` der Domäne werden re-exportiert —
//! `Zeile` trägt ausschließlich adapter-eigene Typen.
//!
//! **Datenschutz (ADR-0008-Reframe):** `einlesen` trägt rohe Werte (IBANs, Klarname) weiter —
//! das Obfuskieren ist der nachgelagerte Slice. Es gibt hier bewusst **keinen**
//! `Serialize`-Guard: rohe Werte dürfen im serialisierbaren Wire-DTO durch die **lokale**
//! Outbox reisen (kein Egress); der Guard wandert auf die Egress-DTOs (auswerten/report).

#[path = "adapter/abstractions/mod.rs"]
mod abstractions;
#[path = "adapter/bridges/consume_bridge/mod.rs"]
mod consume_bridge;
#[path = "adapter/commands/eingelesen/mod.rs"]
mod eingelesen;
#[path = "adapter/commands/eingelesen_message/mod.rs"]
mod eingelesen_message;
#[path = "adapter/bridges/eingelesen_publisher/mod.rs"]
mod eingelesen_publisher;
#[path = "adapter/handler/mod.rs"]
mod handler;
#[path = "adapter/bridges/kalender_bridge/mod.rs"]
mod kalender_bridge;
#[path = "adapter/dto/rohsatz/mod.rs"]
mod rohsatz;
#[path = "adapter/testapi/mod.rs"]
mod testapi;
#[path = "adapter/commands/umsaetze_einlesen/mod.rs"]
mod umsaetze_einlesen;
#[path = "adapter/dto/zeile/mod.rs"]
mod zeile;

pub use abstractions::{KalenderSeam, Kalendertag};
pub use rohsatz::Rohsatz;
pub use testapi::EinlesenTestApi;
pub use zeile::{Richtung, Zeile};
