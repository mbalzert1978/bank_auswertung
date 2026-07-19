//! Black-Box-Spezifikation des fenstern-Slice — ausschließlich über die public API des Adapters
//! (Fluent-`WindowingTestApi` + DTOs + der re-exportierte Domänen-`WindowError`), nie über Interna.
//! `run` fährt die **echte** Kette (`AdvanceWindowCommandDecoder` → `WindowingHandler`) gegen
//! öffentliche Fakes und gibt den **nackten Domänen-Fehler** zurück — keine Outbox-Typen (die
//! Outbox-Mechanik prüft der io-Integrationstest).
//!
//! Der Slice konsumiert `AdvanceWindow{anchor}`, liest die obfuskierten Buchungen der Seite
//! `[anchor − window_days + 1 … anchor]` per Port, reduziert sie **N→1** zu **einem**
//! `Auswertungsfenster`, legt es in eigener Ablage ab (die Ablage vergibt die ID) und publiziert
//! **diese neue ID** als Zeiger. Weil dieser Schnitt **noch nicht paginiert**
//! ([0037](../../../docs/issues/0037-fenstern-selbst-fortsetzung-mehr-seiten-paging.md)), endet die
//! Kette sofort mit einem zusätzlichen `HistoryComplete`.

use fenstern_adapter::{AnalysisWindowRow, WindowError, WindowingTestApi};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{ANKER, UNTERGRENZE, api, erwartetes_fenster, zeile};

// --- Kern-Pfad: N Zeilen → genau ein Fenster, genau ein Zeiger ------------------

#[test]
fn eine_seite_mit_n_buchungen_ergibt_genau_ein_fenster_und_genau_einen_zeiger() {
    // Drei Buchungen innerhalb der Seite, nach Buchungstag aufsteigend (die Lesereihenfolge der
    // Seiten-Query) — die Seite selbst reicht von `UNTERGRENZE` bis zum `ANKER`.
    let api = api()
        .with_obfuscated(800, zeile("buchung-1", "EUR"))
        .with_obfuscated(900, zeile("buchung-2", "EUR"))
        .with_obfuscated(ANKER, zeile("buchung-3", "EUR"));

    let ergebnis = api.run(ANKER);

    assert_eq!(ergebnis, Ok(()));
    // Echte N→1-Reduktion: **ein** Aggregat, das die drei Zeilen nur per Fremdschlüssel einfriert
    // (Snapshot-Stabilität, ADR-0013 Entscheidung A).
    api.assert_stored(erwartetes_fenster(&["buchung-1", "buchung-2", "buchung-3"]));
    // Der Zeiger trägt die **von der Ablage neu vergebene** ID, nicht die einer Quell-Zeile
    // (Tabelle = Wahrheit, Message = Pointer).
    api.assert_window_closed(&["fenster-1"]);
}

// --- Noch keine Paginierung: die Kette endet sofort -----------------------------

#[test]
fn ohne_paginierung_endet_die_kette_sofort_mit_der_lauf_spanne() {
    let api = api().with_obfuscated(ANKER, zeile("buchung-1", "EUR"));

    let ergebnis = api.run(ANKER);

    assert_eq!(ergebnis, Ok(()));
    api.assert_window_closed(&["fenster-1"]);
    // Dieser Schnitt paginiert noch nicht (die Selbst-Fortsetzung kommt in 0037) — der Schritt
    // meldet die Historie darum sofort als erschöpft, mit der Lauf-Spanne. Erst dieser Abschluss
    // lässt `auswerten` seine fenster-übergreifende Reichweite laufen (ADR-0013, Entscheidung E).
    api.assert_history_complete(&[(UNTERGRENZE, ANKER)]);
}

// --- `window_days` kommt als Wert herein und spannt die Seite auf ---------------

#[test]
fn die_seitengroesse_entscheidet_welche_buchungen_ins_fenster_fallen() {
    // Zwei Buchungen, 100 Tage auseinander. Bei einer Seitengröße von 50 Tagen liegt die ältere
    // **unterhalb** der Untergrenze — dieselben Daten, derselbe Anker, ein anderes Fenster.
    let api = WindowingTestApi::new()
        .with_window_days(50)
        .with_obfuscated(ANKER - 100, zeile("zu-alt", "EUR"))
        .with_obfuscated(ANKER, zeile("in-der-seite", "EUR"));

    api.run(ANKER).expect("Seite mit einer Buchung");

    api.assert_stored(AnalysisWindowRow {
        from: ANKER - 49,
        to: ANKER,
        currency: "EUR".to_string(),
        booking_ids: vec!["in-der-seite".to_string()],
    });
}

// --- Invarianten an der Konstruktion: nicht leer, eine Währung ------------------

#[test]
fn eine_seite_ohne_buchung_ergibt_kein_fenster() {
    // Die einzige Buchung liegt außerhalb der Seite — es gibt nichts zu materialisieren. Ein leeres
    // Auswertungsfenster ist unkonstruierbar, der Lauf endet terminal (ADR-0013, Entscheidung B:
    // jedes Fenster sitzt auf echten Daten).
    let api = api().with_obfuscated(UNTERGRENZE - 1, zeile("zu-alt", "EUR"));

    let ergebnis = api.run(ANKER);

    assert_eq!(ergebnis, Err(WindowError::EmptyWindow));
    api.assert_nothing_emitted();
}

#[test]
fn eine_seite_mit_gemischten_waehrungen_ergibt_kein_fenster() {
    // Die Domäne summiert keine gemischten Währungen (ADR-0005) — das Fenster entsteht gar nicht
    // erst, statt hinterher „geprüft" zu werden.
    let api = api()
        .with_obfuscated(900, zeile("in-euro", "EUR"))
        .with_obfuscated(ANKER, zeile("in-franken", "CHF"));

    let ergebnis = api.run(ANKER);

    assert_eq!(ergebnis, Err(WindowError::MixedCurrency));
    api.assert_nothing_emitted();
}

// --- Alle drei I/O-Richtungen sind fehlbar und terminal (ADR-0012) --------------

#[test]
fn ein_fehlschlag_beim_lesen_beendet_den_lauf_terminal() {
    let api = api()
        .with_obfuscated(ANKER, zeile("buchung-1", "EUR"))
        .with_failing_source("Verbindung zur Ablage abgebrochen");

    let ergebnis = api.run(ANKER);

    assert_eq!(
        ergebnis,
        Err(WindowError::SourceUnavailable {
            reason: "Verbindung zur Ablage abgebrochen".to_string()
        })
    );
    api.assert_nothing_emitted();
}

#[test]
fn ein_fehlschlag_beim_ablegen_beendet_den_lauf_terminal() {
    let api = api()
        .with_obfuscated(ANKER, zeile("buchung-1", "EUR"))
        .with_failing_store("Tabelle gesperrt");

    let ergebnis = api.run(ANKER);

    assert_eq!(
        ergebnis,
        Err(WindowError::StorageUnavailable {
            reason: "Tabelle gesperrt".to_string()
        })
    );
    api.assert_nothing_emitted();
}

#[test]
fn ein_fehlschlag_beim_publizieren_beendet_den_lauf_terminal() {
    let api = api()
        .with_obfuscated(ANKER, zeile("buchung-1", "EUR"))
        .with_failing_publisher("Outbox nicht erreichbar");

    let ergebnis = api.run(ANKER);

    // Kein Dead-Letter, kein Retry im Slice — der nackte Domänen-Fehler steigt durch.
    assert_eq!(
        ergebnis,
        Err(WindowError::PublishFailed {
            reason: "Outbox nicht erreichbar".to_string()
        })
    );
    // Die Ablage hat zwar gestaged, aber publiziert wurde nichts; den atomaren Rückbau besorgt io
    // (der Handler ist transaktions-blind).
    api.assert_window_closed(&[]);
    api.assert_history_complete(&[]);
}

// --- Redaktion: die Begründung spiegelt nie einen Rohwert -----------------------

#[test]
fn ein_fehlendes_pflichtfeld_scheitert_terminal_und_redigiert() {
    // Die Zeile liegt in der Ablage, aber ein Pflichtfeld (Währung) ist leer — terminal, und die
    // Begründung trägt **nur den statischen Feldnamen**, nie einen Rohwert.
    let api = api().with_obfuscated(ANKER, zeile("buchung-mit-luecke", ""));

    let ergebnis = api.run(ANKER);

    let fehler = ergebnis.expect_err("erwartet terminalen Fehler");
    assert_eq!(
        fehler,
        WindowError::MissingMandatoryField { field: "currency" }
    );
    assert_eq!(
        fehler.to_string(),
        "mandatory field missing in obfuscated booking: currency"
    );
    // Kein Rohwert im Grund — auch nicht die ID der auslösenden Zeile.
    assert!(!fehler.to_string().contains("buchung-mit-luecke"));
    api.assert_nothing_emitted();
}

#[test]
fn eine_zeile_ohne_id_scheitert_terminal_statt_einen_leeren_zeiger_durchzulassen() {
    // Der Fremdschlüssel ist das, was das Fenster einfriert — fehlt er, gibt es nichts zu
    // referenzieren. Die Präsenz-Prüfung des Adapters weist ihn terminal ab, bevor eine leere ID
    // ins Aggregat und von dort in einen publizierten Zeiger geraten kann.
    let api = api().with_obfuscated(ANKER, zeile("", "EUR"));

    let ergebnis = api.run(ANKER);

    assert_eq!(
        ergebnis,
        Err(WindowError::MissingMandatoryField { field: "id" })
    );
    api.assert_nothing_emitted();
}
