//! einlesen-Slice — **Adapter**: umschließt die blinde Domäne ([`einlesen_domain`]) und ist
//! die **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Er ist der
//! Kopf der event-getriebenen Pipeline: eine `UmsätzeEinlesen`-Nachricht **fächert pro
//! Rohzeile zu je einem `Eingelesen`-Command auf** (kein Batch). Scheitert eine gelesene Zeile
//! am Aggregat, ist das **terminal**: der Handler steigt mit dem Fehler aus, der über die
//! Consume-Bridge zu `OutboxError::Warning` bubblet — die **io-Outbox** dead-lettert die
//! Nachricht (der Slice dead-lettert nicht selbst). Ein transienter Lesefehler der Quelle wird
//! zu `OutboxError::Retry`.
//!
//! **Genau ein öffentlicher Zugang** (Direktive: eine Adapter-Grenze): die
//! [`EinlesenTestApi`], die DTOs ([`Zeile`], [`Rohsatz`]) und die io-facing Ports
//! ([`Rohquelle`]/[`Zeilenspeicher`] + der Lesefehler [`RohquelleError`]) — dazu die vom
//! `Zeile`-DTO getragenen Domänen-Wert-Typen ([`Richtung`]/[`Kalendertag`]). **Alles andere
//! ist crate-privat**: der Handler, die Consume-/Publisher-Bridges, die adapter-lokalen
//! Kommandos (`UmsätzeEinlesen`/`Eingelesen`/`EingelesenMessage`) und die Fakes — keiner dieser
//! Typen ist von außen erreichbar.
//!
//! **Datenschutz (ADR-0008-Reframe):** `einlesen` trägt rohe Werte (IBANs, Klarname) weiter —
//! das Obfuskieren ist der nachgelagerte Slice. Es gibt hier bewusst **keinen**
//! `Serialize`-Guard: rohe Werte dürfen im serialisierbaren Wire-DTO durch die **lokale**
//! Outbox reisen (kein Egress); der Guard wandert auf die Egress-DTOs (auswerten/report).

mod consume_bridge;
mod eingelesen;
mod eingelesen_message;
mod eingelesen_publisher;
mod handler;
mod ports;
mod rohsatz;
mod testapi;
mod umsaetze_einlesen;
mod zeile;

pub use einlesen_domain::{Kalendertag, Richtung};
pub use ports::{Rohquelle, RohquelleError, Zeilenspeicher};
pub use rohsatz::Rohsatz;
pub use testapi::EinlesenTestApi;
pub use zeile::Zeile;
