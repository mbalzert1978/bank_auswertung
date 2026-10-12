//! auswerten-Slice — **Adapter**: umschließt die blinde Domäne ([`auswerten_domain`]) und ist die
//! **einzige** öffentliche Fläche des Slice ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md),
//! [ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
//! [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)). Er ist
//! der vierte Schritt der Pipeline und **die Analyse**. Er ist **restlos outbox-frei**: er bindet
//! **nur** `application_kernel` und nennt keinen Outbox-Typ — die Outbox-Mechanik (Envelope,
//! Consumer-Rolle, `UnitOfWork`) lebt vollständig in `io`.
//!
//! **Zwei Reichweiten, zwei einzweckige Handler (ADR-0013, Entscheidung E) — beide gebaut.**
//!
//! Die **erste** ist die **pro-Fenster**-Reichweite: der [`WindowEvaluationHandler`] konsumiert
//! `FensterGeschlossen` (registriert auf [`CONSUMED_MESSAGE_TYPE`]) — die Nachricht trägt **nur die
//! ID** des eingefrorenen Auswertungsfensters, einen **Zeiger** (uniformes ID-Handoff, ADR-0012).
//! Er **liest** das Aggregat darüber, rechnet die periodische Kennzahl **innerhalb** des Fensters —
//! heute den **Cashflow je Monat** — samt den **distinkten Fingerabdrücken** der Buchungsarten des
//! Fensters und **legt** das Ergebnis ab.
//!
//! Die **zweite** ist die **fenster-übergreifende**: der [`HistoryEvaluationHandler`] konsumiert
//! `HistorieVollständig` (registriert auf [`CONSUMED_HISTORY_MESSAGE_TYPE`] — ein **anderer**
//! Dispatch-Key, ein geschlossenes Fenster erreicht ihn also nie). Der Trigger ist nötig, weil
//! `auswerten` die letzte Seite **nicht selbst erkennen kann**: der Pager läuft neu→alt, und nur
//! `fenstern` weiß, wann die Historie erschöpft ist. Die Nachricht trägt die **Lauf-Spanne** im
//! `partition_key` (`"{from}:{to}"`), die die Query auf **genau diesen Lauf** scopt — ohne sie
//! flössen die Aggregate früherer Läufe **lautlos** mit ein. Er führt **eine** Query über **alle**
//! Fenster des Laufs ([`OccurrenceSeam`]) und **gibt** die [`HistoryEvaluationRow`] **zurück** — die
//! [`OccurrenceRow`]s der ganzen Zeitachse und die darin erkannten **wiederkehrenden Abflüsse**
//! ([`RecurrenceRow`], [Issue 0005](../../../docs/issues/0005-wiederkehr-erkennung.md)): je
//! Kostentreiber ([`CostDriverRow`]) Takt ([`RecurrenceRhythm`]), typischer Betrag, **Jahresbetrag**, ggf. die jüngste
//! Preissteigerung ([`PriceIncreaseRow`]) und das belastete Konto. Er legt **nichts** ab. Kein
//! Vorkommens-/Fold-Log, keine Akkumulation, keine zweite persistierte Sicht — die Aggregate sind
//! die alleinige Wahrheit, und die Idempotenz folgt daraus statt aus Dedupe-Logik: eine Query über
//! unveränderliche Snapshots antwortet auf erneute Zustellung identisch.
//!
//! **Dieselbe Reichweite rechnet den Aderlass** ([Issue 0007](../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)):
//! den genetteten Cashflow je eigenem Konto und Monat ([`AccountCashflowRow`]), das
//! **Pareto-Ranking** der Kostentreiber ([`DriverRankRow`] je [`CostDriverRow`] — Buchungsart,
//! Gebühren eines Kontos oder der Sammel-Treiber ohne Gegenpartei —, mit Anteil und kumuliertem
//! Anteil in Basispunkten und dem **Jahresbetrag**) und die **Spar-Kandidaten**
//! ([`SavingsCandidateRow`]): jeder Treiber, dessen Jahresbetrag die [`SavingsThresholdConfig`]
//! erreicht. Sie steht hier und nicht pro Fenster, weil nur diese Reichweite den ganzen Lauf sieht.
//!
//! **So ruft ein späterer Schritt die Auswertung ab** (Bericht 0012, Chat-Export 0011):
//! [`HistoryEvaluationHandler::new`] mit dem Lese-Seam, der Eigene-Konten-Config und der
//! Spar-Schwelle verdrahten, die
//! Lauf-Spanne über den [`EvaluateHistoryCommandDecoder`] zum Kommando machen (`"{from}:{to}"`, wie
//! `fenstern`s `HistorieVollständig` sie trägt) und `Handler::handle` rufen — die Antwort ist die
//! [`HistoryEvaluationRow`]. Die Wiederkehr arbeitet auf **genetteten** Abflüssen: ein Dauerauftrag
//! aufs eigene Konto ist eine Umbuchung, kein Kosten-Treiber; darum braucht auch diese Reichweite die
//! Eigene-Konten-Config und prüft deren Seed-Marker.
//!
//! **Das Gegenkonto wird auf Token eingeordnet** (ADR-0013, Entscheidung F): der Adapter nimmt die
//! **token-orientierte** Eigene-Konten-Config ([`OwnAccountsConfig`]) als **Wert** entgegen — io
//! verdrahtet sie beim Bau, der Slice liest **keine Datei**. Ihre Deutung „außen → innen" ist die
//! **ACL-Stelle, an der eine fehlerhafte Config abgelehnt wird**: eine unbekannte Rolle oder ein
//! leeres Token sind terminal, nie still übersprungen. Die **IBAN prüft dieser Slice nicht** — sie
//! ist hier nicht mehr vorhanden; ihre Gültigkeit hat der Loader
//! ([Issue 0002](../../../docs/issues/0002-eigene-konten-config-schema-und-loader.md)) **vor** der
//! Tokenisierung geführt, und das Token trägt den Beweis (_parse, don't validate_).
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
//! **Schichtung:** die domänen-internen Handler hängen **nur** an den verlangten Domänen-Ports
//! (`AnalysisWindowSource`/`EvaluationRepository` bzw. `OccurrenceSource`); die Port-Bridges (Modul
//! `adapter`) mappen die öffentlichen, mechanischen Abstractions
//! ([`AnalysisWindowSeam`]/[`EvaluationStoreSeam`]/[`OccurrenceSeam`], die io erfüllt) darauf.
//! **Alle sind fehlbar und terminal** (ADR-0012). Es gibt **keinen** Publish-Seam: die eine
//! Reichweite legt ab, die andere gibt zurück, und keine stößt etwas stromabwärts an.
//!
//! **Öffentliche Fläche:** die beiden Fluent-TestApis ([`EvaluationTestApi`]/[`HistoryTestApi`]),
//! das gelesene Fenster ([`AnalysisWindowRow`]/[`WindowBookingRow`]/[`BookingDirection`]), das
//! abgelegte Ergebnis ([`WindowEvaluationRow`]/[`MonthlyCashflowRow`]/[`CounterAccountRow`]), die
//! zurückgegebene Historien-Auswertung ([`HistoryEvaluationRow`] mit [`OccurrenceRow`]/
//! [`RecurrenceRow`]/[`RecurrenceRhythm`]/[`PriceIncreaseRow`]), die gelesene Lauf-Historie
//! ([`RunHistoryRow`]), die Eigene-Konten-Config
//! ([`OwnAccountsConfig`]/[`OwnAccountRow`]), die von `io` erfüllten Seams samt ihren
//! Seam-Fehlern und die beiden Slice-Verträge ([`WindowEvaluationHandler`] +
//! [`EvaluateWindowCommand`]/[`EvaluateWindowCommandDecoder`]/[`CONSUMED_MESSAGE_TYPE`];
//! [`HistoryEvaluationHandler`] +
//! [`EvaluateHistoryCommand`]/[`EvaluateHistoryCommandDecoder`]/[`CONSUMED_HISTORY_MESSAGE_TYPE`])
//! samt dem re-exportierten [`EvaluationError`]. Alles andere ist crate-privat. Die eingehenden
//! Nachrichten tragen **nur die ID** bzw. **nur die Spanne** — kein Wire-DTO, **kein serde** im
//! Slice.
//!
//! **Was das Einhängen der fenster-übergreifenden Reichweite verlangt:** ihr Ergebnis wird
//! **zurückgegeben, nicht abgelegt** — es lebt allein in der `Handler::Response`. Das ist der Preis
//! dafür, dass es **keine** zweite persistierte Sicht gibt (kein Vorkommens-/Fold-Log): wer diese
//! Reichweite konsumiert, muss die Response **entgegennehmen**, sonst verpufft ihr Ergebnis. io reicht
//! sie darum an einen direkten Abnehmer weiter.
//!
//! **Datenschutz:** der Slice liegt **stromabwärts** der Obfuskierungs-Grenze. Er sieht **nie einen
//! strukturellen Identifikator** — im Sinn von CONTEXT.md („Obfuskierung"): eigene IBAN/Kontonr.,
//! Gegen-IBAN, BIC — und **nie den Seed**; das seed-basierte Pseudonym kommt fertig aus
//! `obfuskieren`, der Seed bleibt dort.
//!
//! Was er sieht, ist opake IDs, eine Währung, Tag-Stamps, Beträge, Konto-/Personen-**Token** und
//! Freitext. Beim Freitext lohnt die Genauigkeit: Verwendungszweck, Buchungstext **und** Firmenname
//! laufen durch den `ObfuskierterText`-Engpass; den **Firmennamen** pseudonymisiert `obfuskieren` dabei
//! **nicht** — bewusst, denn er ist das wichtigste Kategorie-Signal (er benennt eine Firma, nicht
//! eine Person). „Stromabwärts der Grenze" heißt also *tokenisiert und gescrubbt*, nicht *frei von
//! Klartext*.
//!
//! Der **Fingerabdruck** schlüsselt darum auf der Gläubiger-ID, einem Pseudonym oder einem aus
//! gescrubbtem Freitext gebildeten Token — nie auf einem Rohwert.
//!
//! **Der Guard ist eingezogen:** seit dem Kategorisierer-Orakel hat der Slice ein **Egress-DTO**,
//! die [`CategoryRequestRow`] — der eine Punkt, an dem Daten die Maschine verlassen. Der
//! `assert_not_impl_any!`-„kein `Serialize`"-Guard liegt auf ihr, dieselbe Linie, die
//! `obfuskierung` auf `ObfuscatedBooking` zieht. Seit der Wiederkehr-Erkennung trägt ihn auch die
//! [`HistoryEvaluationRow`] samt ihren Teilen: sie ist das, was Bericht und Chat-Export lesen werden,
//! also ein **Egress-Kandidat** — wer sie hinausreichen will, muss das ausdrücklich tun, nicht per
//! `derive`.

mod abstractions;
mod adapter;
mod handler;
mod history_handler;
mod models;
mod testapi;

pub use abstractions::{
    AnalysisWindowSeam, CategorizerSeam, CategorizerUnavailable, CategoryRegistrySeam,
    EvaluationStoreSeam, OccurrenceSeam, RegistryUnavailable, SourceUnavailable, StoreUnavailable,
};
pub use handler::WindowEvaluationHandler;
pub use history_handler::HistoryEvaluationHandler;
pub use models::{
    AccountCashflowRow, AnalysisWindowRow, BookingDirection, BookingRecipient,
    CONSUMED_HISTORY_MESSAGE_TYPE, CONSUMED_MESSAGE_TYPE, CategoryAnswerRow, CategoryRequestRow,
    CategoryRow, CostDriverRow, CounterAccountRow, DriverRankRow, EvaluateHistoryCommand,
    EvaluateHistoryCommandDecoder, EvaluateWindowCommand, EvaluateWindowCommandDecoder, FeeRow,
    HistoryEvaluationRow, InternalTransferRow, MonthlyCashflowRow, OccurrenceRow, OwnAccountRow,
    OwnAccountsConfig, PriceIncreaseRow, RecurrenceRhythm, RecurrenceRow, RunHistoryRow,
    SavingsCandidateRow, SavingsThresholdConfig, WindowBookingRow, WindowEvaluationRow,
    account_role_label,
};
pub use testapi::{EvaluationTestApi, HistoryTestApi};

/// Der **nackte Domänen-Fehler** des Slice, re-exportiert: der [`WindowEvaluationHandler`] gibt ihn
/// 1:1 zurück, `io` spiegelt seine **redigierte** `Display`. Bewusst re-exportiert statt
/// adapter-lokal re-deklariert — anders als die projizierten DTO-Typen ist der Fehler kein
/// Datenwert, sondern der Redaktions-Vertrag selbst: seine `Display` ist der **alleinige**
/// Eigentümer des Meldungssatzes, ein Re-Deklarat wäre ein Satz-Duplikat über Crates.
pub use auswerten_domain::EvaluationError;

/// Die Prüfregel der Gläubiger-ID dieses Slice (Issue 0011), re-exportiert: `obfuskieren` führt
/// dieselbe Regel, und weil die Domänen blind füreinander sind, hält ein io-Test über eine
/// Fall-Tabelle fest, dass beide gleich urteilen.
pub use auswerten_domain::is_valid_creditor_id;

#[cfg(test)]
mod datenschutz {
    use super::{
        AccountCashflowRow, CategoryRequestRow, CostDriverRow, DriverRankRow, HistoryEvaluationRow,
        OccurrenceRow, PriceIncreaseRow, RecurrenceRow, SavingsCandidateRow,
    };
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_not_impl_any;

    // Die Anfrage ans Orakel ist das **Egress-DTO** dieses Slice — hier verlassen Daten die
    // Maschine. Sie darf **nie** serialisieren: einen strukturellen Identifikator in etwas
    // einzubetten, das hinausgeht, muss ein **Compile-Fehler** sein, keine Review-Beobachtung.
    // Dieselbe Linie, die `obfuskierung` an der Datenschutz-Grenze auf `ObfuscatedBooking` zieht.
    //
    // Der Guard steht in einem `#[cfg(test)]`-Modul: er greift über das **Test-Gate** (`cargo test`),
    // nicht schon beim `cargo build`. Seit dem Chat-Export (Issue 0011) ist das die Leitplanke vor dem
    // einen echten Abfluss — die sechs Gates laufen vor jedem Commit.
    assert_not_impl_any!(CategoryRequestRow: Serialize, Deserialize<'static>);

    // Das **Wiederkehr-Ergebnis** ist das, was der Bericht (Issue 0012) und der Chat-Export (Issue
    // 0011) lesen werden — ein Egress-Kandidat. Es trägt nur Fingerabdrücke, Beträge, Tag-Stamps und
    // Konto-Token; dass niemand per `derive` eine Abkürzung nach draußen baut, sichert der Compiler.
    assert_not_impl_any!(HistoryEvaluationRow: Serialize, Deserialize<'static>);
    assert_not_impl_any!(RecurrenceRow: Serialize, Deserialize<'static>);
    assert_not_impl_any!(PriceIncreaseRow: Serialize, Deserialize<'static>);
    assert_not_impl_any!(OccurrenceRow: Serialize, Deserialize<'static>);

    // Der **Aderlass** (Issue 0007) ist genau das, was der Bericht zeigt: Cashflow je Konto, Ranking
    // und Spar-Kandidaten. Token, Fingerabdrücke, Beträge, Monate — und auch dafür keine Abkürzung
    // nach draußen per `derive`.
    assert_not_impl_any!(AccountCashflowRow: Serialize, Deserialize<'static>);
    assert_not_impl_any!(CostDriverRow: Serialize, Deserialize<'static>);
    assert_not_impl_any!(DriverRankRow: Serialize, Deserialize<'static>);
    assert_not_impl_any!(SavingsCandidateRow: Serialize, Deserialize<'static>);
}
