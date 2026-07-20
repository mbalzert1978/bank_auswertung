//! auswerten-Slice — **Adapter**: umschließt die blinde Domäne ([`auswerten_domain`]) und ist die
//! **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
//! [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)). Er ist
//! der vierte Schritt der Pipeline und **die Analyse**. Er ist **restlos outbox-frei**: er bindet
//! **nur** `application_kernel` und nennt keinen Outbox-Typ — die Outbox-Mechanik (Envelope,
//! Consumer-Rolle, `UnitOfWork`) lebt vollständig in `io`.
//!
//! **Zwei Reichweiten, zwei einzweckige Handler (ADR-0013, Entscheidung E).** Gebaut ist die
//! **erste**, die **pro-Fenster**-Reichweite: der [`WindowEvaluationHandler`] konsumiert
//! `FensterGeschlossen` (registriert auf [`CONSUMED_MESSAGE_TYPE`]) — die Nachricht trägt **nur die
//! ID** des eingefrorenen Auswertungsfensters, einen **Zeiger** (uniformes ID-Handoff, ADR-0012).
//! Er **liest** das Aggregat darüber, rechnet die periodische Kennzahl **innerhalb** des Fensters —
//! heute den **Cashflow je Monat** — samt den **distinkten Fingerabdrücken** der Buchungsarten des
//! Fensters und **legt** das Ergebnis ab. Die zweite, fenster-übergreifende Reichweite (Wiederkehr,
//! angestoßen von `HistorieVollständig`) kommt mit ihrem eigenen Handler.
//!
//! **Der Fingerabdruck ist Domänen-, keine Adapter-Sache** (ADR-0004, ADR-0013 Entscheidung F): der
//! Adapter reicht nur die Zellen herein, aus denen er entsteht — die obfuskierte Gegenpartei
//! ([`BookingRecipient`]) und den obfuskierten Verwendungszweck. Die Intermediär-Erkennung und die
//! Bildung des Händler-Tokens liegen **in der Domäne**, nicht hier und nicht in `io`.
//!
//! **Reine Pipeline (ADR-0011, Entscheidung A):** `io`s generische `OutboxConsumer` fährt den
//! Handler: sie extrahiert die Envelope, ruft den slice-gelieferten
//! [`EvaluateWindowCommandDecoder`] (Shape Y, nur nackte Strings; die Fenster-ID liegt im
//! `partition_key`) und dispatcht. Ein terminaler Fehler ist der **nackte Domänen-[`EvaluationError`]**
//! (hier re-exportiert), dessen **redigierte** `Display` `io` spiegelt.
//!
//! **Schichtung:** der domänen-interne `auswerten_domain::WindowEvaluationHandler` hängt **nur** an
//! den verlangten Domänen-Ports (`AnalysisWindowSource`/`EvaluationRepository`); die Port-Bridges
//! (Modul [`adapter`]) mappen die öffentlichen, mechanischen Abstractions
//! ([`AnalysisWindowSeam`]/[`EvaluationStoreSeam`], die io erfüllt) darauf. **Beide sind fehlbar und
//! terminal** (ADR-0012). Es gibt **keinen** Publish-Seam: diese Reichweite legt ab und stößt nichts
//! stromabwärts an.
//!
//! **Öffentliche Fläche:** die Fluent-[`EvaluationTestApi`], das gelesene Fenster
//! ([`AnalysisWindowRow`]/[`WindowBookingRow`]/[`BookingDirection`]), das abgelegte Ergebnis
//! ([`WindowEvaluationRow`]/[`MonthlyCashflowRow`]), die von `io` erfüllten Seams samt ihren
//! Seam-Fehlern und der von `io` verdrahtete Slice-Vertrag ([`WindowEvaluationHandler`],
//! [`EvaluateWindowCommand`]/[`EvaluateWindowCommandDecoder`], [`CONSUMED_MESSAGE_TYPE`], der
//! re-exportierte [`EvaluationError`]). Alles andere ist crate-privat. Die eingehende Nachricht
//! trägt **nur die ID** — kein Wire-DTO, **kein serde** im Slice.
//!
//! **Datenschutz:** der Slice liegt **stromabwärts** der Obfuskierungs-Grenze — er sieht opake IDs,
//! eine Währung, Tag-Stamps, Beträge und den bereits obfuskierten Freitext/Token, nie einen
//! strukturellen Identifikator und **nie den Seed**: das seed-basierte Pseudonym kommt fertig aus
//! `obfuskieren`, der Seed bleibt dort. Auch der Fingerabdruck trägt darum nur die öffentliche
//! Gläubiger-ID oder ein Token, nie einen Rohwert.
//! Darum trägt er heute keinen eigenen `assert_not_impl_any!`-Guard: es gibt hier noch kein
//! Egress-DTO. Der Guard zieht ein, sobald die Reichweite eines bekommt (der `Kategorisierer`-Request
//! und die Report-Ausgabe — dort verlassen Daten die Maschine).

mod abstractions;
mod adapter;
mod handler;
mod models;
mod testapi;

pub use abstractions::{
    AnalysisWindowSeam, EvaluationStoreSeam, SourceUnavailable, StoreUnavailable,
};
pub use handler::WindowEvaluationHandler;
pub use models::{
    AnalysisWindowRow, BookingDirection, BookingRecipient, CONSUMED_MESSAGE_TYPE,
    EvaluateWindowCommand, EvaluateWindowCommandDecoder, MonthlyCashflowRow, WindowBookingRow,
    WindowEvaluationRow,
};
pub use testapi::EvaluationTestApi;

/// Der **nackte Domänen-Fehler** des Slice, re-exportiert: der [`WindowEvaluationHandler`] gibt ihn
/// 1:1 zurück, `io` spiegelt seine **redigierte** `Display`. Bewusst re-exportiert statt
/// adapter-lokal re-deklariert — anders als die projizierten DTO-Typen ist der Fehler kein
/// Datenwert, sondern der Redaktions-Vertrag selbst: seine `Display` ist der **alleinige**
/// Eigentümer des Meldungssatzes, ein Re-Deklarat wäre ein Satz-Duplikat über Crates.
pub use auswerten_domain::EvaluationError;
