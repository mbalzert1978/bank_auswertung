//! io-Integrationstest ([Issue 0045](../../../docs/issues/0045-io-auswerten-end-to-end.md)):
//! ein `StartAnalysis` läuft **end-to-end über zwei Slices** — `fenstern` paginiert und
//! materialisiert die Auswertungsfenster, `auswerten` rechnet je Fenster die periodische Kennzahl
//! und am Schluss, auf `HistorieVollständig`, die fenster-übergreifende Query. Damit ist zum ersten
//! Mal die **volle Pipeline** gegen Fakes lauffähig.
//!
//! Getestet wird die **io-Verdrahtung** (Envelope-Extraktion, `CommandDecoder`-Aufruf, atomarer
//! `UnitOfWork`-Commit, uniforme message_type-Auflösung **aller vier** Kommandotypen, Fehler-Taktik),
//! nicht das Slice-Verhalten — das prüfen die `fenstern`- und `auswerten`-Specs. Alle Seams sind
//! Fakes; der echte SQLite-/`sqlx`-Teil bleibt draußen (Build-Order Schritt 5). Kein Datei-, Netz-
//! oder echter Datenbankzugriff.
//!
//! Die Seam-Fakes, die Fake-Arbeitseinheit, die Dispatcher-Verdrahtung und die Broker-Schleife
//! liegen in den Fixtures nebenan (`fixtures/auswerten/mod.rs`).

use io::outbox::OutboxError;

#[path = "../fixtures/auswerten/mod.rs"]
mod fixtures;
use auswerten_adapter::CounterAccountRow;
use fixtures::{
    FEB_29, FakeBestand, FakeDb, JAN_15, Verdrahtung, bestand, dispatcher_mit, envelope,
    fahre_lauf, kennzahl, monat, nachricht, vorkommen,
};

// --- Der volle Lauf: ein Anstoß → beide Ergebnisse ---------------------------------------

#[test]
fn ein_start_analysis_treibt_die_kette_bis_zu_beiden_ergebnissen() {
    let bestand = bestand();
    let db = FakeDb::default();
    let verdrahtung = Verdrahtung::neu(&bestand, &db);
    let dispatcher = dispatcher_mit(&bestand, &verdrahtung, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    assert_eq!(ergebnis, Ok(()));

    // **Erstes** Ergebnis: je materialisiertem Fenster eine Pro-Fenster-Kennzahl, jede mit dem
    // Fremdschlüssel auf ihr Quell-Fenster. Fenster ≠ Zeitraum — das erste Fenster (30 Tage) zerfällt
    // in **zwei** Monats-Buckets; die Umbuchung landet als Zufluss auf dem eigenen Giro-Konto.
    assert_eq!(
        db.committed_evaluations(),
        vec![
            kennzahl(
                "fenster-1",
                vec![monat(2024, 1, 250_000, 0), monat(2024, 2, 0, 1_999)],
                vec![CounterAccountRow::Own {
                    role: "giro".to_string()
                }],
            ),
            kennzahl("fenster-2", vec![monat(2024, 1, 0, 1_999)], vec![]),
        ]
    );

    // **Zweites** Ergebnis: die fenster-übergreifende Query lief **genau einmal** und legte die
    // Vorkommen **beider** Fenster auf eine Zeitachse — chronologisch, unter demselben
    // Fingerabdruck. Genau das kann die pro-Fenster-Reichweite nicht sehen.
    assert_eq!(
        verdrahtung.senke.lieferungen(),
        vec![vec![vorkommen(JAN_15, 1_999), vorkommen(FEB_29, 1_999)]]
    );

    // Die ganze Kette in der Outbox: `fenstern`s Selbst-Fortsetzung und die beiden Zeiger, an denen
    // `auswerten` hängt — die Slices koppeln allein über diese Nachrichten.
    assert_eq!(
        db.committed_messages(),
        vec![
            nachricht("AdvanceWindow", "19782:19782"),
            nachricht("WindowClosed", "fenster-1"),
            nachricht("AdvanceWindow", "19737:19782"),
            nachricht("WindowClosed", "fenster-2"),
            nachricht("HistoryComplete", "19708:19782"),
        ]
    );
}

// --- Beide auswerten-Kommandotypen: uniform aufgelöst, je einzweckig ---------------------

#[test]
fn beide_auswerten_kommandotypen_werden_vom_selben_dispatcher_aufgeloest() {
    let bestand = bestand();
    let db = FakeDb::default();
    let verdrahtung = Verdrahtung::neu(&bestand, &db);
    let dispatcher = dispatcher_mit(&bestand, &verdrahtung, db.clone());

    // Erst die Fenster materialisieren lassen (der Zeiger, an dem `auswerten` hängt, entsteht dort).
    fahre_lauf(&dispatcher, &db).expect("der Lauf über beide Slices");

    // Ein `WindowClosed` erreicht **nur** die pro-Fenster-Reichweite: es legt eine Kennzahl ab und
    // liefert **nichts** an die Senke — einzweckig.
    let vor_der_zustellung = verdrahtung.senke.lieferungen().len();
    assert_eq!(
        dispatcher.dispatch(envelope(io::auswerten::CONSUMED_MESSAGE_TYPE, "fenster-1")),
        Ok(())
    );
    assert_eq!(
        db.committed_evaluations().len(),
        3,
        "eine Kennzahl kam hinzu"
    );
    assert_eq!(verdrahtung.senke.lieferungen().len(), vor_der_zustellung);

    // Ein `HistoryComplete` erreicht **nur** die fenster-übergreifende: es liefert an die Senke und
    // legt **nichts** ab — es gibt kein Vorkommens-/Fold-Log.
    assert_eq!(
        dispatcher.dispatch(envelope(
            io::auswerten::CONSUMED_HISTORY_MESSAGE_TYPE,
            "19708:19782"
        )),
        Ok(())
    );
    assert_eq!(db.committed_evaluations().len(), 3, "keine neue Kennzahl");
    assert_eq!(
        verdrahtung.senke.lieferungen().len(),
        vor_der_zustellung + 1
    );
}

// --- Die Arbeitseinheit: Ablage und Nachrichten fallen gemeinsam zurück ------------------

#[test]
fn ein_commit_fehler_rollt_kennzahl_und_nachrichten_gemeinsam_zurueck() {
    // Der **dritte** Commit scheitert. Die Schleife arbeitet FIFO, also ist das genau die erste
    // Pro-Fenster-Auswertung: Anstoß (1) und erster Fenster-Schritt (2) committen noch — Letzterer
    // reiht dabei **beide** seiner Nachrichten ein, den `WindowClosed`-Zeiger und die Fortsetzung.
    let bestand = bestand();
    let db = FakeDb::commit_scheitert_ab(3);
    let verdrahtung = Verdrahtung::neu(&bestand, &db);
    let dispatcher = dispatcher_mit(&bestand, &verdrahtung, db.clone());

    let ergebnis = fahre_lauf(&dispatcher, &db);

    // Der Handler gelingt, aber der Commit scheitert → `Retry` (nicht `Warning`).
    assert!(
        matches!(&ergebnis, Err(OutboxError::Retry { reason }) if reason.contains("commit")),
        "ein Commit-Fehler ist der einzige io-interne Retry-Ursprung: {ergebnis:?}"
    );
    // Von der gescheiterten Arbeitseinheit ist **nichts** da: keine Kennzahl. Nur was **davor**
    // sauber committete, steht — die drei Nachrichten der beiden `fenstern`-Schritte.
    assert!(db.committed_evaluations().is_empty());
    assert_eq!(
        db.committed_messages(),
        vec![
            nachricht("AdvanceWindow", "19782:19782"),
            nachricht("WindowClosed", "fenster-1"),
            nachricht("AdvanceWindow", "19737:19782"),
        ]
    );
}

// --- Terminaler Handler-Fehler: redigiert, nichts committet ------------------------------

#[test]
fn ein_ins_leere_zeigender_zeiger_wird_terminal_ueber_die_display_gespiegelt() {
    // Ein Zeiger, der ins Leere zeigt: zu dieser Fenster-ID liegt nichts in der Ablage. Das ist
    // **terminal** — kein Retry, kein Dead-Letter im Slice (ADR-0012), und io spiegelt die
    // **`Display`** des nackten Domänen-Fehlers, nie sein `Debug`.
    let bestand = FakeBestand::default();
    let db = FakeDb::default();
    let verdrahtung = Verdrahtung::neu(&bestand, &db);
    let dispatcher = dispatcher_mit(&bestand, &verdrahtung, db.clone());

    let ergebnis = dispatcher.dispatch(envelope(
        io::auswerten::CONSUMED_MESSAGE_TYPE,
        "fenster-gibt-es-nicht",
    ));

    // Der Grund trägt die **opake** Fenster-ID — bewusst, sie ist ein io-vergebener Zeiger und kein
    // Rohwert (dieselbe Linie, die `obfuskierung` bei der Buchungs-ID zieht). Was er **nicht** trägt,
    // ist Zeilen-/Feldinhalt: der leere Bestand hätte hier nichts beizusteuern, und der Fehler
    // entsteht, bevor eine Buchung gelesen wird.
    assert_eq!(
        ergebnis,
        Err(OutboxError::Warning {
            reason: "referenced analysis window not found: fenster-gibt-es-nicht".to_string()
        })
    );
    assert!(db.committed_evaluations().is_empty());
}

// --- Unlesbarer Zeiger: Decoder `None` → Critical auf Envelope-Ebene ---------------------

#[test]
fn eine_unlesbare_lauf_spanne_wird_critical() {
    let bestand = bestand();
    let db = FakeDb::default();
    let verdrahtung = Verdrahtung::neu(&bestand, &db);
    let dispatcher = dispatcher_mit(&bestand, &verdrahtung, db.clone());

    // Ein `partition_key` ohne die Form `"{from}:{to}"` ist keine gültige Spanne → Decoder `None` →
    // `Critical` (Envelope-Ebene), nicht als Handler-Fehler durchgereicht.
    let ergebnis = dispatcher.dispatch(envelope(
        io::auswerten::CONSUMED_HISTORY_MESSAGE_TYPE,
        "keine-spanne",
    ));

    assert!(
        matches!(&ergebnis, Err(OutboxError::Critical { source, .. })
            if source == io::auswerten::CONSUMED_HISTORY_MESSAGE_TYPE),
        "eine unlesbare Spanne muss zu Critical auf Envelope-Ebene führen: {ergebnis:?}"
    );
    assert!(verdrahtung.senke.lieferungen().is_empty());
}
