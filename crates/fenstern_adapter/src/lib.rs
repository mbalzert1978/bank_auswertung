//! fenstern-Slice — **Adapter**: umschließt die blinde Domäne ([`fenstern_domain`]) und ist die
//! **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
//! [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)). Er ist
//! der dritte Schritt der Pipeline und der **paginierende Analyse-Kopf**. Er ist **restlos
//! outbox-frei**: er bindet **nur** `application_kernel` und nennt keinen Outbox-Typ — die
//! Outbox-Mechanik (Envelope, Consumer-Rolle, `UnitOfWork`) lebt vollständig in `io`.
//!
//! **Zwei einzweckige Handler (ADR-0013, Entscheidung B/D):** der [`StartHandler`] eröffnet den Lauf
//! — er konsumiert das **feldlose** [`StartAnalysisCommand`] (registriert auf
//! [`START_MESSAGE_TYPE`]), beschafft den Initial-Anker über den Lese-Seam (`max(booking_day)`,
//! datenrelativ, keine Uhr) und publiziert damit das erste Fenster-Kommando. Ein **leerer Bestand**
//! ist dabei **kein Fehler**: der Lauf endet erfolgreich, ohne dass etwas publiziert wird — und weil
//! so kein `HistoryComplete` mit leerer Spanne entsteht, bleibt dessen Lauf-Spanne nicht entartet.
//! Der [`WindowingHandler`] materialisiert danach je Kommando ein Fenster. **Zwei** Kommandotypen
//! statt eines mit optionalem Anker: der **Typ trägt die Absicht** (ADR-0005).
//!
//! **Reine Pipeline (ADR-0011, Entscheidung A):** der [`WindowingHandler`] ist der
//! `application_kernel::Handler` des Slice — `AdvanceWindowCommand → Domäne → Response`. `io`s
//! generische `OutboxConsumer` fährt ihn: sie extrahiert die Envelope, ruft den slice-gelieferten
//! [`AdvanceWindowCommandDecoder`] (Shape Y, nur nackte Strings; der **Anker** liegt als
//! stringifizierter Tag-Stamp im `partition_key`) und dispatcht. Der Handler spannt die Seite
//! `[anchor − window_days + 1 … anchor]` auf, **liest** ihre obfuskierten Zeilen, **reduziert** sie
//! **N→1** zu einem Auswertungsfenster, **legt** es ab (die Persistenz vergibt die neue ID) und
//! **publiziert genau diese neue ID** als Zeiger (Tabelle = Wahrheit, Message = Pointer). Ein
//! terminaler Fehler ist der **nackte Domänen-[`WindowError`]** (hier re-exportiert), dessen
//! **redigierte** `Display` `io` spiegelt.
//!
//! **Schichtung:** der domänen-interne `fenstern_domain::WindowingHandler` hängt **nur** an den
//! verlangten Domänen-Ports (`ObfuscatedWindowSource`/`AnalysisWindowRepository`/
//! `WindowClosedPublisher`); die Port-Bridges (Modul [`adapter`]) mappen die öffentlichen,
//! mechanischen Abstractions ([`ObfuscatedWindowSeam`]/[`AnalysisWindowStoreSeam`]/
//! [`WindowPublishSeam`], die io erfüllt) darauf. **Alle drei sind fehlbar und terminal** (ADR-0012).
//!
//! **Öffentliche Fläche:** die Fluent-[`WindowingTestApi`], die obfuskierte Zeile
//! [`ObfuscatedRow`] (die der Lese-Seam liefert), die abgelegte [`AnalysisWindowRow`], die von `io`
//! erfüllten Seams samt ihren Seam-Fehlern und der von `io` verdrahtete Slice-Vertrag
//! ([`WindowingHandler`], [`AdvanceWindowCommand`]/[`AdvanceWindowCommandDecoder`], die opaken
//! [`WindowClosedMessage`]/[`HistoryCompleteMessage`], [`CONSUMED_MESSAGE_TYPE`], der re-exportierte [`WindowError`]). Alles
//! andere ist crate-privat. Die eingehende Nachricht trägt **nur den Anker** — kein Wire-DTO,
//! **kein serde** im Slice.
//!
//! **Datenschutz:** der Slice liegt **stromabwärts** der Obfuskierungs-Grenze — er sieht nur opake
//! IDs, eine Währung und Tag-Stamps, nie einen strukturellen Identifikator oder Freitext. Darum
//! trägt er keinen eigenen `assert_not_impl_any!`-Guard: es gibt hier kein Egress-DTO, in das eine
//! rohe Kennung überhaupt geraten könnte.

mod abstractions;
mod adapter;
mod handler;
mod models;
mod start_handler;
mod testapi;

pub use abstractions::{
    AnalysisWindowStoreSeam, ObfuscatedWindowSeam, PublishUnavailable, SourceUnavailable,
    StoreUnavailable, WindowPublishSeam,
};
pub use handler::WindowingHandler;
pub use models::{
    AdvanceWindowCommand, AdvanceWindowCommandDecoder, AdvanceWindowMessage, AnalysisWindowRow,
    CONSUMED_MESSAGE_TYPE, HistoryCompleteMessage, ObfuscatedRow, START_MESSAGE_TYPE,
    StartAnalysisCommand, StartAnalysisCommandDecoder, WindowClosedMessage,
};
pub use start_handler::StartHandler;
pub use testapi::WindowingTestApi;

/// Der **nackte Domänen-Fehler** des Slice, re-exportiert: der [`WindowingHandler`] gibt ihn 1:1
/// zurück, `io` spiegelt seine **redigierte** `Display`. Bewusst re-exportiert statt adapter-lokal
/// re-deklariert — anders als die projizierten DTO-Typen ist der Fehler kein Datenwert, sondern der
/// Redaktions-Vertrag selbst: seine `Display` ist der **alleinige** Eigentümer des Meldungssatzes,
/// ein Re-Deklarat wäre ein Satz-Duplikat über Crates.
pub use fenstern_domain::WindowError;
