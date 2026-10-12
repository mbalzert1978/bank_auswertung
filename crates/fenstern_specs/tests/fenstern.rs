//! Black-Box-Spezifikation des fenstern-Slice — ausschließlich über die public API des Adapters
//! (Fluent-`WindowingTestApi` + DTOs + der re-exportierte Domänen-`WindowError`), nie über Interna.
//! `run` fährt die **echte** Kette (`AdvanceWindowCommandDecoder` → `WindowingHandler`) gegen
//! öffentliche Fakes und gibt den **nackten Domänen-Fehler** zurück — keine Outbox-Typen (die
//! Outbox-Mechanik prüft der io-Integrationstest).
//!
//! Der Slice hat **zwei** einzweckige Handler und je ein eigenes Kommando. Der **Anstoß**
//! konsumiert das **ankerlose** `StartAnalysis` (es trägt nur die Lauf-Identität, ADR-0016) (`start`/`start_chain`): er beschafft den
//! Initial-Anker über den Anker-Port (`max(booking_day)` — datenrelativ, **keine Uhr**, darum in den
//! Specs allein über `with_obfuscated` arrangierbar) und publiziert damit das erste Fenster-Kommando.
//! Auf **leerem** Bestand endet er **erfolgreich**, ohne irgendetwas zu publizieren.
//!
//! Der Fenster-Schritt konsumiert `AdvanceWindow{anchor, run_top}`, liest die obfuskierten Buchungen der Seite
//! `[anchor − window_days + 1 … anchor]` per Port, reduziert sie **N→1** zu **einem**
//! `Auswertungsfenster`, legt es in eigener Ablage ab (die Ablage vergibt die ID) und publiziert
//! **diese neue ID** als Zeiger. Danach **setzt er sich selbst fort**: der nächste Anker ist
//! `max(booking_day < from)` — liefert die Abfrage einen Tag, geht die Kette weiter, sonst schließt
//! sie mit `HistoryComplete` und der Lauf-Spanne ab (ADR-0013, Entscheidung B, Präzisierung 2).
//! `run` fährt **einen** Schritt, `run_chain` den **ganzen** Lauf (die Rolle, die in Produktion die
//! Outbox spielt).

use fenstern_adapter::{AnalysisWindowRow, WindowError, WindowingTestApi};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{ANKER, UNTERGRENZE, WINDOW_DAYS, api, erwartetes_fenster, fenster, zeile};

// --- Der Anstoß: StartAnalysis → genau ein AdvanceWindow -----------------------

#[test]
fn ein_anstoss_ergibt_genau_ein_fenster_kommando_mit_dem_juengsten_buchungstag_als_anker() {
    // Der Anstoß ist **ankerlos** — er trägt keinen Anker, er *beschafft* ihn: der Start-Handler
    // liest ihn einmalig über den Anker-Port (`max(booking_day)`), also **datenrelativ** und
    // deterministisch, ohne Uhr (ADR-0013, Entscheidung D).
    let api = api()
        .with_obfuscated(800, zeile("alt", "EUR"))
        .with_obfuscated(ANKER, zeile("jung", "EUR"));

    let ergebnis = api.start();

    assert_eq!(ergebnis, Ok(()));
    // Genau **ein** Fenster-Kommando, und sein Anker ist der jüngste Buchungstag des Bestands.
    api.assert_advance_window(&[ANKER]);
    // Der Anstoß ist einzweckig: er materialisiert **kein** Fenster und stößt `auswerten` nicht an
    // — das ist Sache des Fenster-Handlers, den das publizierte Kommando erst auslöst.
    api.assert_stored_windows(&[]);
    api.assert_window_closed(&[]);
    api.assert_history_complete(&[]);
}

#[test]
fn ein_anstoss_auf_leerem_bestand_endet_erfolgreich_und_publiziert_nichts() {
    // Kein einziger Buchungstag: der Anker-Port liefert `None`. Das ist die **gültige** Antwort
    // „leerer Bestand", kein Fehler — Erschöpfung schon bei Schritt 0, symmetrisch zum normalen
    // Ketten-Ende. Kein Dead-Letter (den gibt es nicht), kein „nichts auszuwerten"-Fehler.
    let api = api();

    let ergebnis = api.start();

    assert_eq!(ergebnis, Ok(()));
    // Es geht **nichts** heraus — die Kette beginnt gar nicht erst.
    api.assert_advance_window(&[]);
    api.assert_stored_windows(&[]);
    api.assert_window_closed(&[]);
    // Insbesondere entsteht **kein** `HistoryComplete`: `auswerten` wird nicht angestoßen, und weil
    // kein Abschluss mit leerer Spanne entsteht, bleibt jede publizierte Lauf-Spanne garantiert
    // nicht entartet.
    api.assert_history_complete(&[]);
}

#[test]
fn der_anstoss_setzt_den_lauf_gipfel_sodass_die_lauf_spanne_nicht_entartet() {
    // Zwei Seiten bei Seitengröße 10. Der Anstoß gibt dem ersten Fenster-Kommando Anker **und**
    // Lauf-Gipfel — beide der jüngste Buchungstag. Genau dieser Gipfel reist unverändert durch die
    // ganze Selbst-Fortsetzung und bildet am Ende die **Obergrenze** der Lauf-Spanne.
    let api = WindowingTestApi::new()
        .with_window_days(10)
        .with_obfuscated(1000, zeile("jung", "EUR"))
        .with_obfuscated(900, zeile("alt", "EUR"));

    let ergebnis = api.start_chain();

    assert_eq!(ergebnis, Ok(()));
    // Das erste Kommando stammt vom Anstoß, das zweite von der Selbst-Fortsetzung.
    api.assert_advance_window(&[1000, 900]);
    // Die Lauf-Spanne reicht von der Unterkante der letzten Seite bis zum Gipfel des Anstoßes —
    // nicht entartet, weil der Gipfel ein echter Buchungstag ist.
    api.assert_history_complete(&[(891, 1000)]);
}

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

// --- Selbst-Fortsetzung: der eine Mechanismus `max(booking_day < from)` ---------

#[test]
fn eine_einzige_seite_endet_sofort_mit_der_lauf_spanne() {
    let api = api().with_obfuscated(ANKER, zeile("buchung-1", "EUR"));

    let ergebnis = api.run(ANKER);

    assert_eq!(ergebnis, Ok(()));
    api.assert_window_closed(&["fenster-1"]);
    // Unter der Fensterunterkante liegt keine Buchung mehr — `max(booking_day < from)` versiegt,
    // also endet die Kette statt sich fortzusetzen. Erst dieser Abschluss lässt `auswerten` seine
    // fenster-übergreifende Reichweite laufen (ADR-0013, Entscheidung E).
    api.assert_advance_window(&[]);
    api.assert_history_complete(&[(UNTERGRENZE, ANKER)]);
}

#[test]
fn eine_historie_ueber_mehrere_seiten_ergibt_je_seite_ein_fenster_und_genau_einen_abschluss() {
    // Drei Buchungen, je eine pro Seite: der nächste Anker ist stets der **jüngste echte
    // Buchungstag unter der Fensterunterkante** — kein Zähler, kein Boolean.
    //   Seite 1 = [601 … 1000] → Buchung 1000, nächster Anker max(< 601) = 500
    //   Seite 2 = [101 …  500] → Buchung  500, nächster Anker max(< 101) = 100
    //   Seite 3 = [-299 … 100] → Buchung  100, max(< -299) versiegt → Abschluss
    let api = api()
        .with_obfuscated(ANKER, zeile("seite-1", "EUR"))
        .with_obfuscated(500, zeile("seite-2", "EUR"))
        .with_obfuscated(100, zeile("seite-3", "EUR"));

    let ergebnis = api.run_chain(ANKER);

    assert_eq!(ergebnis, Ok(()));
    // Je Seite genau ein abgelegtes Fenster und genau ein Zeiger stromabwärts.
    api.assert_window_closed(&["fenster-1", "fenster-2", "fenster-3"]);
    // Die Kette hat sich zweimal selbst fortgesetzt — die Anker sind echte Buchungstage.
    api.assert_advance_window(&[500, 100]);
    // **Genau ein** Abschluss, und er trägt die **Lauf-Spanne**: von der Unterkante der letzten
    // Seite bis zum Anker, mit dem der Lauf begann (nicht die Spanne der letzten Seite).
    api.assert_history_complete(&[(100 - WINDOW_DAYS as i64 + 1, ANKER)]);
}

#[test]
fn eine_datenluecke_groesser_als_eine_seitengroesse_wird_uebersprungen() {
    // 100 Tage Lücke bei einer Seitengröße von 10 — der ältere Anker ist ein **echter Buchungstag**,
    // nicht `from − 1`. Die leere Zwischenzone wird darum gar nicht erst zu Seiten: es entstehen
    // **zwei** Fenster statt zehn, keines davon leer, und die Kette bricht nicht ab.
    let api = WindowingTestApi::new()
        .with_window_days(10)
        .with_obfuscated(1000, zeile("jung", "EUR"))
        .with_obfuscated(900, zeile("alt", "EUR"));

    let ergebnis = api.run_chain(1000);

    assert_eq!(ergebnis, Ok(()));
    api.assert_advance_window(&[900]);
    api.assert_stored_windows(&[fenster(991, 1000, &["jung"]), fenster(891, 900, &["alt"])]);
    api.assert_history_complete(&[(891, 1000)]);
}

#[test]
fn die_fenster_ueberlappen_nicht_und_jede_buchung_landet_in_genau_einem() {
    // Fünf Buchungen, Seitengröße 10. Weil der nächste Anker **echt unterhalb** der Unterkante
    // liegt, stoßen die Seiten lückenlos aneinander, ohne sich zu überschneiden.
    let api = WindowingTestApi::new()
        .with_window_days(10)
        .with_obfuscated(1000, zeile("b-1000", "EUR"))
        .with_obfuscated(995, zeile("b-995", "EUR"))
        .with_obfuscated(991, zeile("b-991", "EUR"))
        .with_obfuscated(990, zeile("b-990", "EUR"))
        .with_obfuscated(985, zeile("b-985", "EUR"));

    api.run_chain(1000).expect("zwei Seiten");

    // [981 … 990] und [991 … 1000] berühren sich, überlappen aber nicht — und zusammen tragen sie
    // jede der fünf Buchungen **genau einmal**.
    api.assert_stored_windows(&[
        fenster(991, 1000, &["b-991", "b-995", "b-1000"]),
        fenster(981, 990, &["b-985", "b-990"]),
    ]);
}

#[test]
fn die_kette_terminiert_weil_der_anker_streng_monoton_sinkt() {
    // Seitengröße 1 — die engstmögliche Seite, also die meisten Fortsetzungen. Zwei Buchungen am
    // **selben** Tag zeigen dabei, dass der nächste Anker **echt** unterhalb liegt (`< from`, nicht
    // `<= from`): ein Tag kann sich nicht selbst als nächsten Anker liefern, sonst liefe die Kette
    // ewig. `run_chain` würde ein Nicht-Terminieren als Fehlschlag melden.
    let api = WindowingTestApi::new()
        .with_window_days(1)
        .with_obfuscated(1000, zeile("b-1000-a", "EUR"))
        .with_obfuscated(1000, zeile("b-1000-b", "EUR"))
        .with_obfuscated(999, zeile("b-999", "EUR"))
        .with_obfuscated(990, zeile("b-990", "EUR"));

    let ergebnis = api.run_chain(1000);

    assert_eq!(ergebnis, Ok(()));
    // Streng monoton sinkend, und der Vorrat versiegt — daher die garantierte Terminierung.
    api.assert_advance_window(&[999, 990]);
    api.assert_history_complete(&[(990, 1000)]);
}

// --- Ablage + beide Publishes sind eine Einheit: keine gegabelte Kette ----------

#[test]
fn je_schritt_feuert_genau_einer_der_beiden_fortsetzungs_zweige() {
    // Fortsetzung und Abschluss schließen einander aus. Solange die Historie trägt, geht **nur**
    // die Fortsetzung heraus — ein zusätzlicher Abschluss wäre die gegabelte Kette.
    let api = api()
        .with_obfuscated(ANKER, zeile("seite-1", "EUR"))
        .with_obfuscated(500, zeile("seite-2", "EUR"));

    api.run(ANKER).expect("erste Seite");

    api.assert_window_closed(&["fenster-1"]);
    api.assert_advance_window(&[500]);
    api.assert_history_complete(&[]);
}

#[test]
fn ein_fehlschlag_beim_publizieren_setzt_die_kette_nicht_fort() {
    // Auch mit tragender Historie geht **keine** Fortsetzung heraus, wenn das Einreihen scheitert:
    // Ablage und beide Publishes sind eine Einheit — es entsteht keine halb-fortgesetzte Kette.
    // Den atomaren Rückbau des Stagings besorgt io (der Handler ist transaktions-blind).
    let api = api()
        .with_obfuscated(ANKER, zeile("seite-1", "EUR"))
        .with_obfuscated(500, zeile("seite-2", "EUR"))
        .with_failing_publisher("Outbox nicht erreichbar");

    let ergebnis = api.run(ANKER);

    assert_eq!(
        ergebnis,
        Err(WindowError::PublishFailed {
            reason: "Outbox nicht erreichbar".to_string()
        })
    );
    api.assert_advance_window(&[]);
    api.assert_history_complete(&[]);
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
        run: WindowingTestApi::DEFAULT_RUN.to_string(),
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
    api.assert_advance_window(&[]);
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

// --- Lauf-Identität: jedes Fenster gehört zu genau einem Lauf (ADR-0016) -----------------
// Jeder Lauf paginiert die **ganze** Historie neu. Damit ein späterer Lauf die Fenster eines
// früheren nicht mitzählt, trägt jedes Fenster und jede Nachricht der Kette die Identität des
// Laufs, den der Anstoß eröffnet hat — unverändert vom Anstoß bis zum Abschluss.

#[test]
fn die_lauf_identitaet_reist_vom_anstoss_bis_in_jedes_fenster_und_den_abschluss() {
    let api = WindowingTestApi::new()
        .with_run("lauf-7")
        .with_window_days(10)
        .with_obfuscated(1000, zeile("juengste", "EUR"))
        .with_obfuscated(900, zeile("aeltere", "EUR"));

    api.start_chain().expect("zwei Seiten, ein Lauf");

    api.assert_stored_windows(&[
        AnalysisWindowRow {
            run: "lauf-7".to_string(),
            ..fenster(991, 1000, &["juengste"])
        },
        AnalysisWindowRow {
            run: "lauf-7".to_string(),
            ..fenster(891, 900, &["aeltere"])
        },
    ]);
    api.assert_every_message_carries_run("lauf-7");
}
