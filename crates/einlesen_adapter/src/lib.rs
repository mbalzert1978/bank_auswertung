//! einlesen-Slice — **Adapter**: umschließt die blinde Domäne ([`einlesen_domain`]) und ist
//! die **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Er ist der
//! Kopf der event-getriebenen Pipeline: eine `ReadTransactions`-Nachricht ist reiner
//! **Auslöser** (feldlose Payload; die Quelle lebt im Envelope-`partition_key`) — den Batch
//! **beschafft** der Handler über den Datenquellen-Port, den die [`SourceBridge`](bridges)
//! an den Quell-Seam bindet
//! ([Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md))
//! — und **fächert pro Rohzeile zu je einem „Eingelesen"-Command auf** (kein Batch-Command
//! stromabwärts). Scheitert die Beschaffung (inkl. Präsenz-Prüfung hinter dem Seam) oder eine
//! beschaffte Zeile am Aggregat, ist das **terminal**: der Handler steigt mit dem Fehler aus,
//! der über die Consume-Bridge zu `OutboxError::Warning` bubblet — die **io-Outbox**
//! dead-lettert die Nachricht (der Slice dead-lettert nicht selbst).
//!
//! **Schichtung (die eine Aufgabe dieses Layers):** ein interner [`ReadHandler`](einlesen_domain::ReadHandler)
//! hängt **ausschließlich** an den von `einlesen_domain` verlangten Ports (`RawLineSource`,
//! `Calendar`, `Money`, `Store`, `EventSink`) — nie an adapter-erfundenen Ports. **Darüber**
//! mappen Port-Bridges (Modul [`bridges`]) die öffentlichen, mechanischen Abstractions (Modul
//! [`abstractions`], die IO erfüllt) auf genau diese internen Domänen-Ports. Vorbild:
//! `examples/Downloader` (`DownloaderHandler` hängt nur an `IFilePuller`/`IDownloadLedger`;
//! `SftpPullerAdapter` mappt den öffentlichen `ISftpPuller`-Seam darauf).
//!
//! **Genau ein öffentlicher Zugang** (Direktive: eine Adapter-Grenze): die [`ReadTestApi`],
//! die DTOs ([`Line`], [`RawRecord`]) und die öffentlichen Abstractions
//! ([`CalendarSeam`] + sein Rückgabetyp [`CalendarDay`], [`MoneySeam`] + seine Rückgabetypen
//! [`DecodedAmount`]/[`Sign`]) — dazu die adapter-eigenen [`Direction`]/[`Currency`].
//! **Alles andere ist crate-privat**: der Handler, alle Bridges, das adapter-lokale Wire-DTO
//! (`IngestedMessage`; eingehend gibt es keins — die `ReadTransactions`-Nachricht ist inhaltsloser
//! Auslöser, die Consume-Bridge baut das Kommando direkt aus dem Envelope) und die Fakes — keiner
//! dieser Typen ist von außen erreichbar. Weder `Direction` noch `CalendarDay` der Domäne werden re-exportiert —
//! `Line` trägt ausschließlich adapter-eigene Typen.
//!
//! **Datenschutz (ADR-0008-Reframe):** `einlesen` trägt rohe Werte (IBANs, Klarname) weiter —
//! das Obfuskieren ist der nachgelagerte Slice. Es gibt hier bewusst **keinen**
//! `Serialize`-Guard: rohe Werte dürfen **lokal** existieren — sie reisen **in-process** über
//! den Quell-Seam in die Domäne (`RawRecord`, seit
//! [0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
//! ohne serde: es reist nicht mehr per Outbox-Nachricht) und liegen in der unter der
//! Aggregat-ID persistierten Roh-Erfassung (`Line`) —, das ist kein Egress. Die ausgehende
//! „Eingelesen"-Nachricht trägt dagegen **nur die ID** als Zeiger (kein Rohwert, kein serde,
//! [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)):
//! der `obfuskieren`-Consumer liest die Nutzlast per ID aus dem einlesen-Speicher. Der
//! `Serialize`-Guard wandert auf die Egress-DTOs (auswerten/report).

mod abstractions;
mod bridges;
mod commands;
mod dto;
mod testapi;

pub use abstractions::{CalendarDay, CalendarSeam, DecodedAmount, MoneySeam, Sign};
pub use dto::{Currency, Direction, Line, RawRecord};
pub use testapi::ReadTestApi;
