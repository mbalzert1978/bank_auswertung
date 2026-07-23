//! io-Integrationstest ([Issue 0039](../../../docs/issues/0039-io-fenstern-end-to-end-ueber-generische-outboxconsumer.md)):
//! ein `StartAnalysis` läuft **end-to-end** durch die generische Consumer-Maschinerie in `io` → den
//! echten `fenstern`-Slice → über mehrere Seiten bis zum abschließenden `HistoryComplete`. Getestet
//! wird die **io-Verdrahtung** (Envelope-Extraktion, `CommandDecoder`-Aufruf, atomarer
//! `UnitOfWork`-Commit, uniforme message_type-Auflösung **beider** Kommandotypen, Fehler-Taktik),
//! nicht das Slice-Verhalten (das prüft der `fenstern`-Spec). Alle Seams sind Fakes; der echte
//! SQLite-/`rusqlite`-Teil bleibt draußen (Build-Order Schritt 5).
//!
//! Die Seam-Fakes, die Fake-Arbeitseinheit, die Dispatcher-Verdrahtung und die Broker-Schleife
//! liegen in den Fixtures nebenan (`fixtures/fenstern/mod.rs`).

use io::config::FensterConfig;
use io::outbox::OutboxError;

#[path = "../fixtures/fenstern/mod.rs"]
mod fixtures;
use fixtures::{
    FakeDb, FakeSource, bestand, dispatcher_mit, envelope, fahre_lauf, fenster, nachricht, zeile,
};

/// Dieselbe gültige Fixture-Config wie im Loader-Test (`fenstern_config.rs`), hier als Beleg, dass
/// der aus der toml **geladene** Wert real in den `fenstern`-Schritt einspeist.
const VALID_CONFIG: &str = include_str!("../fixtures/fenstern_config/valid.toml");

// --- Happy Path: StartAnalysis → drei Seiten → HistorieVollständig ----------------------

#[test]
fn ein_start_analysis_laeuft_ueber_mehrere_seiten_bis_historie_vollstaendig() {
    let source = bestand();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    assert_eq!(ergebnis, Ok(()));
    // Je Seite **ein** abgelegtes Auswertungsfenster, neu→alt, jeweils nur mit den Fremdschlüsseln
    // der Zeilen, die auf der Seite liegen.
    assert_eq!(
        db.committed_windows(),
        vec![
            fenster(98, 100, &["b-1", "b-2"]),
            fenster(93, 95, &["b-3"]),
            fenster(88, 90, &["b-4"]),
        ]
    );
    // Die ganze Kette in der Outbox: der Anstoß eröffnet, jede Seite schließt ihr Fenster und reiht
    // die nächste ein, und **genau ein** `HistoryComplete` mit der Lauf-Spanne schließt ab. Der
    // Lauf-Gipfel (100) reist unverändert durch alle Anker.
    assert_eq!(
        db.committed_messages(),
        vec![
            nachricht("AdvanceWindow", "100:100"),
            nachricht("WindowClosed", "fenster-1"),
            nachricht("AdvanceWindow", "95:100"),
            nachricht("WindowClosed", "fenster-2"),
            nachricht("AdvanceWindow", "90:100"),
            nachricht("WindowClosed", "fenster-3"),
            nachricht("HistoryComplete", "88:100"),
        ]
    );
}

// --- Config-Loader speist die Seitengröße in den Fenster-Schritt (Issue 0054) ------------

#[test]
fn die_aus_der_toml_geladene_seitengroesse_paginiert_entlang_der_buchungstag_achse() {
    // Der Loader liest `window_days` aus der Datei; genau dieser Wert geht — als Wert, nicht als Datei
    // — in den `fenstern`-Schritt, der entlang der fixierten Buchungstag-Achse (ADR-0013 C) paginiert.
    let config = FensterConfig::from_toml_str(VALID_CONFIG).expect("gültige Config lädt");
    let source = bestand();
    let db = FakeDb::default();

    let mut dispatcher = io::Dispatcher::new();
    assert!(
        dispatcher
            .register(
                io::fenstern::CONSUMED_MESSAGE_TYPE,
                Box::new(io::fenstern::consumer(
                    config.window_days(),
                    &source,
                    db.clone()
                )),
            )
            .is_none(),
        "frische Registrierung darf keinen Consumer verdrängen"
    );

    assert_eq!(
        dispatcher.dispatch(envelope(io::fenstern::CONSUMED_MESSAGE_TYPE, "100:100")),
        Ok(())
    );

    // Seitengröße 3 aus der toml → Seite `[100 − 3 + 1 … 100] = [98 … 100]`; die Untergrenze 98 ist
    // genau `anchor − window_days + 1`, beweist also, dass der geladene Wert die Achsen-Arithmetik trägt.
    assert_eq!(
        db.committed_windows(),
        vec![fenster(98, 100, &["b-1", "b-2"])]
    );
}

// --- Beide Kommandotypen: uniforme Auflösung über den einen Dispatcher -------------------

#[test]
fn beide_kommandotypen_werden_vom_selben_dispatcher_aufgeloest() {
    let source = bestand();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    // Der Anstoß: aufgelöst, verarbeitet — er publiziert nur das erste Fenster-Kommando und legt
    // selbst **nichts** ab (einzweckig).
    assert_eq!(
        dispatcher.dispatch(envelope(io::fenstern::START_MESSAGE_TYPE, "")),
        Ok(())
    );
    assert!(db.committed_windows().is_empty());
    assert_eq!(
        db.committed_messages(),
        vec![nachricht("AdvanceWindow", "100:100")]
    );

    // Der Fenster-Schritt: derselbe Dispatcher, anderer `message_type`, anderer Handler — er
    // materialisiert das Fenster.
    assert_eq!(
        dispatcher.dispatch(envelope(io::fenstern::CONSUMED_MESSAGE_TYPE, "100:100")),
        Ok(())
    );
    assert_eq!(
        db.committed_windows(),
        vec![fenster(98, 100, &["b-1", "b-2"])]
    );
}

// --- Leerer Bestand: erfolgreich, ohne jede Nachricht ------------------------------------

#[test]
fn ein_leerer_bestand_endet_erfolgreich_ohne_nachricht() {
    let source = FakeSource::default();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    // Erschöpfung schon bei Schritt 0: kein Fehler, aber auch kein `HistoryComplete` mit entarteter
    // Spanne — `auswerten` wird gar nicht erst angestoßen.
    assert_eq!(ergebnis, Ok(()));
    assert!(db.committed_windows().is_empty());
    assert!(db.committed_messages().is_empty());
}

// --- Terminaler Handler-Fehler: redigiert, nichts committet, Kette bricht ab -------------

#[test]
fn ein_terminaler_fehler_rollt_die_ganze_arbeitseinheit_zurueck() {
    // Pflichtfeld leer (Währung) auf der **zweiten** Seite → `MissingMandatoryField` (terminal).
    let source = bestand().mit_zeile_am(95, zeile("b-3", ""));
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    // Der Grund trägt nur den statischen Feldnamen, nie eine Buchungs-ID.
    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason })
            if reason.contains("currency") && !reason.contains("b-3")),
        "erwartet redigiertes terminales Dead-Letter: {ergebnis:?}"
    );
    // Die erste Seite steht (sie committete sauber); von der zweiten ist **nichts** da — weder das
    // Fenster noch der `WindowClosed`-Zeiger noch eine Fortsetzung. Keine halb-fortgesetzte Kette.
    assert_eq!(
        db.committed_windows(),
        vec![fenster(98, 100, &["b-1", "b-2"])]
    );
    assert_eq!(
        db.committed_messages(),
        vec![
            nachricht("AdvanceWindow", "100:100"),
            nachricht("WindowClosed", "fenster-1"),
            nachricht("AdvanceWindow", "95:100"),
        ]
    );
}

// --- Commit-Fehler: der einzige Retry-Fall, nichts committet -----------------------------

#[test]
fn ein_commit_fehler_hinterlaesst_keine_halb_fortgesetzte_kette() {
    // Der **zweite** Commit scheitert: der Anstoß committet noch, der erste Fenster-Schritt nicht.
    let source = bestand();
    let db = FakeDb::commit_scheitert_ab(2);
    let dispatcher = dispatcher_mit(&source, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    // Der Handler gelingt, aber der Commit scheitert → `Retry` (nicht `Warning`).
    assert!(
        matches!(&ergebnis, Err(OutboxError::Retry { reason }) if reason.contains("commit")),
        "ein Commit-Fehler ist der einzige io-interne Retry-Ursprung: {ergebnis:?}"
    );
    // Ablage **und** beide Publishes des gescheiterten Schrittes fielen gemeinsam zurück: nur die
    // Nachricht des Anstoßes steht, kein Fenster.
    assert!(db.committed_windows().is_empty());
    assert_eq!(
        db.committed_messages(),
        vec![nachricht("AdvanceWindow", "100:100")]
    );
}

// --- Blanker Anker: Decoder `None` → Critical auf Envelope-Ebene -------------------------

#[test]
fn ein_ankerloser_partition_key_wird_critical() {
    let source = bestand();
    let db = FakeDb::default();
    let dispatcher = dispatcher_mit(&source, db.clone());

    // Ein `partition_key` ohne die Form `"{anchor}:{run_top}"` ist kein gültiger Anker → Decoder
    // `None` → `Critical` (Envelope-Ebene), nicht als Handler-Fehler durchgereicht.
    let ergebnis = dispatcher.dispatch(envelope(io::fenstern::CONSUMED_MESSAGE_TYPE, "   "));

    assert!(
        matches!(&ergebnis, Err(OutboxError::Critical { source, .. })
            if source == io::fenstern::CONSUMED_MESSAGE_TYPE),
        "ein ankerloser Zeiger muss zu Critical auf Envelope-Ebene führen: {ergebnis:?}"
    );
    assert!(db.committed_windows().is_empty());
}
