//! Black-Box-Spezifikation der **zweiten, fenster-übergreifenden** Reichweite des auswerten-Slice
//! ([ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
//! Entscheidung E, Reichweite 2) — ausschließlich über die public API des Adapters
//! (Fluent-`HistoryTestApi` + DTOs + der re-exportierte Domänen-`EvaluationError`), nie über Interna.
//!
//! Anders als die pro-Fenster-Reichweite hängt dieser Handler an **`HistorieVollständig`**, nicht an
//! `FensterGeschlossen`: `auswerten` kann die letzte Seite **nicht selbst erkennen** — der Pager
//! läuft neu→alt, und nur `fenstern` weiß, wann die Historie erschöpft ist.
//!
//! Er läuft **einmal**, führt **eine Read-Port-Query über alle Auswertungsfenster des Laufs** und
//! liefert deren **Vorkommen** (Fingerabdruck, Richtung, Betrag, Tag) über die ganze Zeitachse
//! zurück — **synchron und terminal**. Er legt dabei **nichts** ab: es gibt kein Vorkommens-/
//! Fold-Log und keine zweite persistierte Sicht; die Aggregate sind die alleinige Wahrheit. Die
//! Rhythmus-/Wiederkehr-Logik darauf ([0005](../../../docs/issues/0005-wiederkehr-erkennung.md))
//! spezifiziert `wiederkehr.rs`; dieser Spec hält die **Vorkommen** fest, auf denen sie aufsetzt.

use auswerten_adapter::{
    BookingDirection, CONSUMED_HISTORY_MESSAGE_TYPE, CONSUMED_MESSAGE_TYPE, EvaluationError,
};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    FEB_01, FEB_29, HAENDLER, JAN_15, JAN_31, MAR_05, ausgang, eingang, eingang_wiederkehrend,
    fenster, historie_api, vorkommen, wiederkehrend,
};

// --- Die Reichweite: über alle Fenster des Laufs, nicht über eines -------------

#[test]
fn ein_historie_vollstaendig_liefert_die_vorkommen_ueber_alle_fenster_des_laufs() {
    // Zwei **verschiedene** eingefrorene Fenster desselben Laufs, jedes mit einer Buchung derselben
    // Art. Genau das kann die pro-Fenster-Reichweite nicht sehen: sie bekommt ein Fenster und ist
    // fertig. Erst diese Query legt die beiden Vorkommen auf **eine** Zeitachse — die Voraussetzung
    // dafür, dass 0005 später einen Rhythmus darin erkennen kann.
    let api = historie_api()
        .with_window("fenster-1", fenster(&[wiederkehrend(JAN_15, 1_999)]))
        .with_window("fenster-2", fenster(&[wiederkehrend(FEB_29, 1_999)]));

    let vorkommen_der_historie = api
        .run_history(JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    // Beide Fenster fließen ein, chronologisch geordnet, und jedes Vorkommen trägt genug für
    // Rhythmus **und** Betragsvergleich: Fingerabdruck, Richtung, Betrag, Tag.
    assert_eq!(
        vorkommen_der_historie,
        Ok(vec![
            vorkommen(
                "COMPANY:DE98ZZZ09999999999",
                JAN_15,
                1_999,
                BookingDirection::Outgoing
            ),
            vorkommen(
                "COMPANY:DE98ZZZ09999999999",
                FEB_29,
                1_999,
                BookingDirection::Outgoing
            ),
        ])
    );
}

// --- Die Lauf-Spanne als Scope: dieser Lauf, nicht ein früherer -----------------

#[test]
fn die_lauf_spanne_grenzt_den_lauf_gegen_frueher_ab() {
    // Zwei Fenster, aber sie gehören **verschiedenen Läufen**: das eine liegt in der Spanne, die das
    // `HistorieVollständig` mitträgt, das andere davor — es stammt aus einem früheren Import.
    //
    // Ohne die Spanne flösse es lautlos mit ein, und der Rhythmus, den 0005 darauf erkennt, wäre der
    // einer Zeitachse, die es nie gab. Der Fehler wäre **still**: es käme ein Ergebnis heraus, nur
    // das falsche. Genau darum trägt `fenstern` die Spanne mit — `auswerten` kann sie nicht selbst
    // kennen, weil der Pager neu→alt läuft.
    let api = historie_api()
        .with_window("lauf-davor", fenster(&[wiederkehrend(JAN_15, 999)]))
        .with_window("dieser-lauf", fenster(&[wiederkehrend(FEB_29, 1_999)]));

    let vorkommen_der_historie = api
        .run_history(FEB_01, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    // Nur das Fenster **dieses** Laufs fließt ein.
    assert_eq!(
        vorkommen_der_historie,
        Ok(vec![vorkommen(
            &format!("COMPANY:{HAENDLER}"),
            FEB_29,
            1_999,
            BookingDirection::Outgoing
        )])
    );
}

#[test]
fn ein_spaeterer_lauf_zaehlt_die_fenster_eines_frueheren_nicht_mit() {
    // Der Fall, den die Spanne allein **nicht** abfängt (ADR-0016): jeder Lauf paginiert die
    // **ganze** Historie neu, also liegen die Fenster des früheren Laufs in **derselben** Spanne wie
    // die des späteren. Nur die Lauf-Identität im `HistorieVollständig` grenzt sie ab — sonst zählte
    // jede Buchung je Lauf einmal mehr.
    let api = historie_api()
        .with_window_of_run(
            "lauf-1",
            "fenster-frueher",
            fenster(&[wiederkehrend(FEB_29, 1_999)]),
        )
        .with_window_of_run(
            "lauf-2",
            "fenster-jetzt",
            fenster(&[wiederkehrend(FEB_29, 1_999)]),
        );

    let vorkommen_der_historie = api
        .run_history_of("lauf-2", JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    assert_eq!(
        vorkommen_der_historie,
        Ok(vec![vorkommen(
            &format!("COMPANY:{HAENDLER}"),
            FEB_29,
            1_999,
            BookingDirection::Outgoing
        )])
    );
}

#[test]
fn beide_grenzen_der_lauf_spanne_zaehlen_mit() {
    // Die Spanne ist **einschließlich** — so setzt `fenstern` sie, und eine Buchung genau auf der
    // Kante gehört zum Lauf. Ein `<`/`<=`-Fehler an dieser Stelle verlöre still den Rand-Tag.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[wiederkehrend(JAN_15, 100), wiederkehrend(MAR_05, 300)]),
    );

    let vorkommen_der_historie = api
        .run_history(JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    assert_eq!(
        vorkommen_der_historie,
        Ok(vec![
            vorkommen(
                &format!("COMPANY:{HAENDLER}"),
                JAN_15,
                100,
                BookingDirection::Outgoing
            ),
            vorkommen(
                &format!("COMPANY:{HAENDLER}"),
                MAR_05,
                300,
                BookingDirection::Outgoing
            ),
        ])
    );
}

// --- Idempotenz ohne Dedupe-Maschinerie ---------------------------------------

#[test]
fn erneute_zustellung_desselben_signals_liefert_dasselbe_ergebnis() {
    // At-least-once heißt: dasselbe `HistorieVollständig` kann **mehrfach** ankommen. Das Ergebnis
    // muss identisch bleiben — und zwar **ohne** Dedupe-/Idempotenz-Maschinerie: es gibt hier nichts
    // zu deduplizieren, weil nichts akkumuliert wird. Die Query läuft über **unveränderliche**
    // Snapshots und ist damit von Natur aus deterministisch.
    let api = historie_api()
        .with_window("fenster-1", fenster(&[wiederkehrend(JAN_15, 1_999)]))
        .with_window("fenster-2", fenster(&[wiederkehrend(FEB_29, 1_999)]));

    let erste_zustellung = api
        .run_history(JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);
    let zweite_zustellung = api
        .run_history(JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    // Identisch — und **nicht verdoppelt**. Genau das belegt zugleich, dass keine zweite
    // persistierte Sicht mitwächst: gäbe es ein Vorkommens-/Fold-Log, stünden nach dem zweiten Lauf
    // vier Vorkommen darin statt zwei.
    assert_eq!(erste_zustellung, zweite_zustellung);
    assert_eq!(erste_zustellung.map(|vorkommen| vorkommen.len()), Ok(2));
}

// --- Die Abgrenzung der beiden Reichweiten ------------------------------------

#[test]
fn ein_fenster_geschlossen_stoesst_diese_reichweite_nicht_an() {
    // Die beiden Reichweiten hängen an **verschiedenen** Dispatch-Keys, und das ist die ganze
    // Abgrenzung: `io`s Prozessor löst auf den Nachrichtentyp auf, ein `FensterGeschlossen` erreicht
    // diesen Handler also nie.
    //
    // Liefe die fenster-übergreifende Query je geschlossenem Fenster, sähe sie nie mehr als das
    // eine Fenster, das gerade fertig wurde — und liefe zudem einmal **pro Seite** statt **einmal**
    // pro Lauf.
    assert_eq!(CONSUMED_HISTORY_MESSAGE_TYPE, "HistoryComplete");
    assert_ne!(CONSUMED_HISTORY_MESSAGE_TYPE, CONSUMED_MESSAGE_TYPE);
}

// --- Was ein Vorkommen trägt --------------------------------------------------

#[test]
fn ein_vorkommen_traegt_richtung_und_betrag_fuer_den_vergleich() {
    // Rhythmus **und** Betragsvergleich: derselbe Händler, dreimal, mit **steigendem** Betrag und
    // einmal in der Gegenrichtung (eine Rückerstattung). Beides muss am Vorkommen ablesbar sein —
    // sonst könnte 0005 weder eine Preiserhöhung erkennen noch eine Gutschrift von einer Zahlung
    // unterscheiden.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            wiederkehrend(JAN_15, 1_999),
            wiederkehrend(FEB_29, 2_499),
            eingang_wiederkehrend(MAR_05, 2_499),
        ]),
    );

    let vorkommen_der_historie = api
        .run_history(JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    let schluessel = format!("COMPANY:{HAENDLER}");
    assert_eq!(
        vorkommen_der_historie,
        Ok(vec![
            vorkommen(&schluessel, JAN_15, 1_999, BookingDirection::Outgoing),
            // Die Preiserhöhung steht als schlichter Betragsunterschied da …
            vorkommen(&schluessel, FEB_29, 2_499, BookingDirection::Outgoing),
            // … und die Rückerstattung unterscheidet sich **nur** in der Richtung, bei gleichem,
            // vorzeichenlosem Betrag (ADR-0005: das Vorzeichen lebt allein in der Richtung).
            vorkommen(&schluessel, MAR_05, 2_499, BookingDirection::Incoming),
        ])
    );
}

#[test]
fn eine_entgeltzeile_ohne_gegenpartei_traegt_kein_vorkommen_bei() {
    // Eine Zeile ohne Gegenpartei hat keinen Schlüssel, über den ihre *Art* unterscheidbar wäre. Ein
    // Platzhalter-Schlüssel verklumpte alle Entgeltzeilen zu einer scheinbar wiederkehrenden Art —
    // die Wiederkehr-Erkennung meldete dann einen Rhythmus, den es nicht gibt.
    let api = historie_api().with_window(
        "fenster-1",
        fenster(&[
            ausgang(JAN_15, 4_200),
            eingang(JAN_31, 250_000),
            wiederkehrend(FEB_29, 1_999),
        ]),
    );

    let vorkommen_der_historie = api
        .run_history(JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    // Nur die Buchung **mit** Gegenpartei trägt bei.
    assert_eq!(
        vorkommen_der_historie,
        Ok(vec![vorkommen(
            &format!("COMPANY:{HAENDLER}"),
            FEB_29,
            1_999,
            BookingDirection::Outgoing
        )])
    );
}

// --- Der eine fehlbare I/O-Schritt --------------------------------------------

#[test]
fn eine_leere_lauf_spanne_ist_kein_fehler_sondern_eine_leere_sicht() {
    // Ein Lauf, in dessen Spanne nichts liegt, ist eine **gültige Auskunft** — anders als beim Lesen
    // *eines* Fensters per ID gibt es hier keinen Zeiger, der ins Leere zeigen könnte. „Nichts
    // vorhanden" und „Beschaffung misslungen" auseinanderzuhalten ist der Punkt.
    let api = historie_api().with_window("lauf-davor", fenster(&[wiederkehrend(JAN_15, 999)]));

    let vorkommen_der_historie = api
        .run_history(FEB_01, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    assert_eq!(vorkommen_der_historie, Ok(vec![]));
}

#[test]
fn ein_fehlschlag_der_beschaffung_endet_den_lauf_terminal() {
    // Der eine fehlbare I/O-Schritt dieser Reichweite. Er endet **terminal** — kein Dead-Letter,
    // kein Retry im Slice (ADR-0012); die io-Diagnose wird durchgereicht, ohne Zeilen-/Feldinhalt.
    let api = historie_api()
        .with_window("fenster-1", fenster(&[wiederkehrend(JAN_15, 1_999)]))
        .with_failing_source("query timed out");

    let vorkommen_der_historie = api
        .run_history(JAN_15, MAR_05)
        .map(|auswertung| auswertung.occurrences);

    assert_eq!(
        vorkommen_der_historie,
        Err(EvaluationError::SourceUnavailable {
            reason: "query timed out".to_string()
        })
    );
}
