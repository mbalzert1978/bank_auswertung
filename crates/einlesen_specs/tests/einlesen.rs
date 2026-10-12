//! Black-Box-Spezifikation des einlesen-Slice — nur über die **eine** öffentliche
//! Adapter-Fläche (Fluent-`ReadTestApi`, das `RawRecord`-DTO und der re-exportierte Domänen-
//! `ReadError`), nie über Interna. Seit dem Kernel-Flip ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md))
//! spezifiziert der Spec auf **Handler-Ebene** über den `application_kernel`-Vertrag: `run`
//! fährt die **echte** Kette (`ReadCommandDecoder` → `ReadingHandler`: Quell-Bridge →
//! Kalender-/Geld-Bridge + interner `ReadHandler` → Publisher-Bridge) gegen öffentliche Fakes
//! und gibt den **nackten Domänen-`ReadError`** zurück — **keine** `OutboxError`-Asserts mehr
//! (die Outbox-Mechanik prüft der io-Integrationstest).
//!
//! **Der maßgebliche Ablauf:** das Kommando ist reiner **Auslöser** (Issue 0018); den Batch
//! (alle Rohzeilen) **beschafft** der Handler über den Datenquellen-Port, hinter dem der
//! Quell-Seam liegt — `with_line` arrangiert die Zeilen darum **in der Quelle**, nicht in der
//! Nachricht. Die Domäne deutet **pro Zeile** ein `LineAggregate` — scheitert die Beschaffung,
//! eine Zeile am Aggregat oder die Ablage/Veröffentlichung, ist das **terminal**: der Handler
//! steigt mit dem `ReadError` aus (der Slice gibt seinen nackten terminalen Fehler zurück). Die
//! gültigen Aggregate werden erst **nach** vollständigem Durchlauf (`collect`) abgelegt; den
//! atomaren Commit besitzt `io`.
//!
//! **Warum es KEIN gemischtes „einige verworfen, andere laufen durch"-Szenario gibt**
//! (bewusst bestätigt, nicht getestet): Das ist kein konstruierbarer Zustand. Es ist
//! **alles-oder-nichts** — die erste terminal ungültige Zeile lässt den ganzen Lauf aussteigen
//! (nichts wird gestaged), sonst laufen alle Zeilen durch.
//!
//! **Feld-Treue der `Line`-Projektion** (originalgetreue Übernahme **aller** Felder inkl.
//! Währung, Pflicht-/Optional-Verhalten) ist ein **Blackbox-Szenario**: `RawRecord` arrangieren,
//! `run` **einmal real** fahren, die gestagete `Line` am Speicher-Seam (`stored_lines`)
//! assertieren — kein Unit-Test der Projektion, sondern beobachtetes Pipeline-Verhalten. Das
//! ausgehende DTO wird am **konkreten** Seam-Typ beobachtet, nicht per generischem Serialisieren.
//!
//! **Was hier sehr wohl abgelehnt wird — und warum das Slice-Verhalten ist:** die *Präsenz*
//! eines Pflichtfelds erzwingt der **Adapter** (ACL, „außen → innen": `RawRecord` → `RawLine`)
//! — seit Issue 0018 **hinter dem Quell-Seam in der Source-Bridge**, weiterhin batch-atomar:
//! fehlt eins, scheitert der **ganze** Batch redigiert (nur Feldname, nie ein Rohwert). Ebenso
//! sichert die Bridge **eine Währung über den ganzen Batch** zu (formatunabhängige
//! Inbound-Zusicherung der ACL, nicht des io-Readers) — ein Bruch scheitert redigiert und
//! batch-atomar (`SourceError::MixedCurrency`, datenlos). Beides ist beobachtbares
//! Slice-Verhalten, kein io-Detail — darum prüfen Specs es. Ebenso sind die Abbildung „Quelle
//! nicht beschaffbar → terminaler Fehler mit durchgereichter io-Diagnose" (Bridge-Verhalten,
//! kein io-Format-Detail) und „unbekannte Währung" (eine **Domänen**-Deutungsentscheidung,
//! `Currencies::resolve`) Slice-Verhalten. Weiterhin **nicht** getestet wird reine io-Mechanik
//! (Datei lesen, echtes Dekodieren von deutschem Datum/Zahlenformat) — die liegt hinter den Seams.
//!
//! **`RawRecord` trägt keine Test-Ergonomie** (bewusst): die Fixtures liegen spec-lokal in
//! `fixtures/` (per Modul-Include hereingezogen), nicht in der Adapter-API.

use einlesen_adapter::{Line, ReadTestApi};

#[path = "../fixtures/mod.rs"]
mod fixtures;
use fixtures::{
    QUELLE, erwartete_zeile, feld, gültiger_rohsatz, rohsatz_mit_schlüssel, vorgemerkter_rohsatz,
};

// --- Eine gültige Zeile: genau ein Aggregat + genau ein Command -------------------

#[test]
fn eine_gültige_zeile_wird_zu_einem_eingelesen_command() {
    let api = ReadTestApi::new().with_line(gültiger_rohsatz());

    api.run(QUELLE).expect("gültige Zeile");

    // Genau **ein** „Eingelesen"-Command, und sein Zeiger deckt sich mit dem Speicher-Schlüssel
    // (Tabelle = Wahrheit, Message = Pointer): der Spec vergleicht die beiden Datenrückgaben der
    // TestApi, ohne die intern vergebene Aggregat-ID als Literal zu benennen.
    assert_eq!(api.published_message_types(), vec!["Ingested"]);
    assert_eq!(api.stored_ids(), api.published_partition_keys());
}

// --- Fan-out: pro Zeile genau ein Command (kein Batch stromabwärts) --------------

#[test]
fn mehrere_zeilen_fächern_zu_je_einem_command_auf() {
    // Drei Zeilen, die sich nur im belanglosen Empfängernamen unterscheiden — deklarativ als
    // `fold` über die Namen gebaut, statt jede Zeile einzeln zu mutieren.
    let api = ["EDEKA", "REWE", "ALDI"]
        .into_iter()
        .fold(ReadTestApi::new(), |api, name| {
            let mut rohsatz = rohsatz_mit_schlüssel(name);
            rohsatz.recipient_name = feld(name);
            api.with_line(rohsatz)
        });

    api.run(QUELLE).expect("gültige Zeilen");

    // Drei „Eingelesen"-Commands (Fan-out), und jeder Zeiger zeigt auf genau die unter diesem
    // Schlüssel abgelegte Roh-Zeile (Tabelle = Wahrheit, Message = Pointer). Weil die Speicher-
    // Schlüssel je Zeile verschieden sind, belegt die Deckung zugleich, dass die drei Zeiger
    // paarweise verschieden sind — ohne dass der Spec an die interne Fake-Nummerierung koppelt.
    assert_eq!(api.published_message_types(), vec!["Ingested"; 3]);
    assert_eq!(api.stored_ids(), api.published_partition_keys());
    // Aggregat **und** Ereignis werden atomar gestaged (ADR-0008): je gültiger Zeile liegt ihr
    // Auftragskonto am Speicher-Seam — in Lesereihenfolge, nicht nur der Fan-out stromabwärts.
    assert_eq!(
        api.stored_ordering_accounts(),
        vec!["DE89370400440532013000"; 3]
    );
}

// --- Feld-Treue: die gestagete Zeile übernimmt alle Felder originalgetreu ----------
// Realer Durchlauf, Beobachtung am konkreten `Line`-DTO des Speicher-Seams (`stored_lines`):
// jedes projizierte Feld — inkl. der **Währung** neben dem Minor-Unit-Betrag — kommt originalgetreu an.

#[test]
fn die_gestagte_zeile_übernimmt_alle_felder_originalgetreu() {
    let api = ReadTestApi::new().with_line(gültiger_rohsatz());

    api.run(QUELLE).expect("gültige Zeile");

    // Die gestagete `Line` als Ganzes gegen die erwartete Projektion — ein `assert_eq!` statt
    // Feld-für-Feld, inkl. der Währung neben dem Minor-Unit-Betrag.
    assert_eq!(api.stored_lines(), vec![erwartete_zeile()]);
}

// --- Feld-Treue: fehlende Optionalfelder werden als `none` gestaged ----------------

#[test]
fn fehlende_optionalfelder_werden_als_none_gestaged() {
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.counter_account = None;
    rohsatz.purpose = None;
    rohsatz.booking_text = None;
    let api = ReadTestApi::new().with_line(rohsatz);

    api.run(QUELLE).expect("Pflichtfelder gesetzt");

    // Erwartung = dieselbe Projektion, nur die drei Optionalfelder auf `None` (Struct-Update);
    // der Ganz-`Line`-Vergleich beweist zugleich, dass **nur** diese Felder `none` werden.
    let erwartet = Line {
        counter_account: None,
        purpose: None,
        booking_text: None,
        ..erwartete_zeile()
    };
    assert_eq!(api.stored_lines(), vec![erwartet]);
}

// --- Redaktion: der terminale Fehlergrund spiegelt keinen Rohwert ------------------
// Scheitert eine Zeile terminal, gibt der Handler seinen nackten `ReadError` zurück, dessen
// `Display` io spiegelt. Hier zählt **das Verhalten der Domäne**, nicht die io-seitige Format-/
// Vollständigkeitsprüfung (die die Domäne fordert, nicht selbst leistet): der Grund darf den
// Rohwert nie spiegeln.

#[test]
fn der_terminale_fehlergrund_spiegelt_den_rohwert_nicht() {
    // Spaltenverschub: eine IBAN landet im Betragsfeld. Die `Display` des terminalen `ReadError`
    // wird io als Fehlergrund persistieren — sie darf den Rohwert nicht spiegeln. Die Redaktion
    // ist Domänen-Invariante; hier prüfen wir, dass sie bis in die Fehlermeldung trägt.
    let leck = "DE89370400440532013000";
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.amount = feld(&format!("-{leck}"));
    let api = ReadTestApi::new().with_line(rohsatz);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(error) if !error.to_string().contains(leck)),
        "der terminale Fehlergrund leakt den Rohwert: {ergebnis:?}"
    );
}

// --- Präsenz: ein fehlendes Pflichtfeld lässt den GANZEN Batch terminal scheitern --------
// Die Präsenz eines Pflichtfelds erzwingt der Adapter (ACL) — seit Issue 0018 in der
// Source-Bridge hinter dem Quell-Seam. Fehlt eins irgendwo, scheitert die „außen → innen"-
// Abbildung schon bei der Beschaffung, bevor der Handler eine Zeile deutet — batch-atomar:
// nichts wird gestaged/veröffentlicht, auch nicht die gültige Zeile daneben. Der Grund ist
// redigiert (nur der Feldname).

#[test]
fn ein_fehlendes_pflichtfeld_scheitert_batch_atomar() {
    let gültige = gültiger_rohsatz();
    let mut ohne_betrag = rohsatz_mit_schlüssel("zeile-2");
    ohne_betrag.amount = None;
    let api = ReadTestApi::new().with_line(gültige).with_line(ohne_betrag);

    let ergebnis = api.run(QUELLE);

    // Terminaler Fehler, dessen Grund nur den Feldnamen benennt (kein Rohwert).
    assert!(
        matches!(&ergebnis, Err(error) if error.to_string().contains("amount")),
        "erwartet redigierter Fehler (nur Feldname): {ergebnis:?}"
    );
    // Batch-atomar: nichts wird gestaged oder veröffentlicht — auch die gültige Zeile nicht.
    assert!(api.stored_lines().is_empty());
    assert!(api.published_message_types().is_empty());
}

// --- Ein-Währung: ein Währungsbruch über den Batch scheitert terminal + redigiert --------
// Die Domäne summiert keine gemischten Währungen; die „eine Währung über den ganzen Batch"-
// Zusicherung ist eine **formatunabhängige** Inbound-Zusicherung der ACL (Source-Bridge, hinter
// dem Quell-Seam) — genau wie die Präsenz-Prüfung, und damit beobachtbares Slice-Verhalten. Zwei
// sonst gültige Zeilen mit verschiedenen Währungen lassen den ganzen Batch terminal scheitern,
// bevor der Handler eine Zeile summiert. Redigiert: der Grund nennt keinen Währungscode.

#[test]
fn ein_währungsbruch_über_den_batch_scheitert_terminal_redigiert() {
    let eur = gültiger_rohsatz(); // trägt bereits EUR
    let mut usd = rohsatz_mit_schlüssel("zeile-2");
    usd.currency = feld("USD");
    let api = ReadTestApi::new().with_line(eur).with_line(usd);

    let ergebnis = api.run(QUELLE);

    // Terminal + redigiert: die Meldung nennt nur die Mechanik, nie einen Währungscode.
    assert!(
        matches!(&ergebnis, Err(error)
            if !error.to_string().contains("EUR") && !error.to_string().contains("USD")),
        "erwartet redigierter Währungsbruch-Fehler ohne Code: {ergebnis:?}"
    );
    // Batch-atomar: nichts wird gestaged oder veröffentlicht — auch die EUR-Zeile nicht.
    assert!(api.stored_lines().is_empty());
    assert!(api.published_message_types().is_empty());
}

// --- Ein-Währung: gleiche Währung in anderer Schreibung ist KEIN Bruch -------------------
// Die Zusicherung teilt die Deutung des Domänen-Decodes (`Currencies::resolve`, case-insensitiv):
// `eur` und `EUR` sind dieselbe Währung, kein Bruch. Beide Zeilen laufen durch.

#[test]
fn gleiche_währung_in_anderer_schreibung_ist_kein_bruch() {
    let groß = gültiger_rohsatz(); // trägt EUR
    let mut klein = rohsatz_mit_schlüssel("zeile-2");
    klein.currency = feld("eur");
    let api = ReadTestApi::new().with_line(groß).with_line(klein);

    api.run(QUELLE)
        .expect("gleiche Währung, nur andere Schreibung");

    // Kein Bruch: beide Zeilen werden gestaged und fächern zu je einem Command auf.
    assert_eq!(api.stored_lines().len(), 2);
    assert_eq!(api.published_message_types(), vec!["Ingested"; 2]);
}

// --- Optionaler Empfänger: eine Gebührenzeile ohne Empfänger läuft durch -----------------
// Eine Entgelt-/Gebührenzeile hat keine Gegenpartei (B2); der Empfängername ist optional. Die
// Zeile wird **nicht** abgewiesen, sondern mit `recipient_name == None` gestaged. Der
// Ganz-`Line`-Vergleich beweist zugleich, dass **nur** dieses Feld `None` wird.

#[test]
fn eine_gebührenzeile_ohne_empfänger_wird_ohne_namen_gestaged() {
    let mut gebühr = gültiger_rohsatz();
    gebühr.recipient_name = None;
    let api = ReadTestApi::new().with_line(gebühr);

    api.run(QUELLE)
        .expect("optionaler Empfänger fehlt — kein Fehler");

    let erwartet = Line {
        recipient_name: None,
        ..erwartete_zeile()
    };
    assert_eq!(api.stored_lines(), vec![erwartet]);
}

// --- Beschaffung: eine nicht beschaffbare Quelle scheitert terminal ----------------------
// Ein Fehlschlag der Beschaffung ist **nie Abwesenheit**, sondern terminal (Issue 0018): die
// Source-Bridge mappt den Seam-Fehler auf den domänen-typisierten `SourceError::Unavailable`.
// Der Grund trägt die **durchgereichte io-Diagnose** (Nachvollziehbarkeit ist beobachtbares
// Verhalten, kein Detail); die reale Lese-Mechanik dahinter bleibt untestiert (io-Sache).
// Nichts wird abgelegt, nichts publiziert.

#[test]
fn eine_nicht_beschaffbare_quelle_scheitert_terminal() {
    let diagnose = "CSV-Datei nicht lesbar: auszug-juni.csv (permission denied)";
    let api = ReadTestApi::new().with_unavailable_source(diagnose);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(error) if error.to_string().contains(diagnose)),
        "erwartet terminaler Fehler mit durchgereichter io-Diagnose: {ergebnis:?}"
    );
    // Terminal und leer: nichts wird abgelegt oder veröffentlicht.
    assert!(api.stored_lines().is_empty());
    assert!(api.published_message_types().is_empty());
}

// --- Minor Units: eine Nicht-2-Stellen-Währung skaliert währungs-korrekt (ISO 4217) ------
// Der Geldwert trägt **währungs-korrekte Minor Units**, nicht die feste Skala 2 (Cent). JPY hat
// 0 Nachkommastellen: „1234 JPY" sind 1234 Minor Units (nicht 123 400). Verifiziert end-to-end
// über den Bridge-Weg — die Minor-Unit-Skalierung lebt in der `MoneyBridge` (`MoneySeam` ×
// `CurrencySeam`), hier real gegen die währungs-bewussten Fakes gefahren. Der ISO-4217-Exponent
// selbst kommt in `io` aus der battle-tested Lib; hier zählt die korrekte Skalierung end-to-end.

#[test]
fn ein_jpy_betrag_skaliert_auf_null_nachkommastellen() {
    let mut jpy = gültiger_rohsatz();
    jpy.amount = feld("+1234");
    jpy.currency = feld("JPY");
    let api = ReadTestApi::new().with_line(jpy);

    api.run(QUELLE).expect("gültige JPY-Zeile");

    // 1234 JPY = 1234 Minor Units (Skala 0), nicht 123 400 wie unter fester Skala 2.
    let lines = api.stored_lines();
    assert!(
        matches!(lines.as_slice(),
            [line] if line.amount_minor_units == 1234 && line.currency == "JPY"),
        "JPY skaliert auf 0 Nachkommastellen: {lines:?}"
    );
}

// --- Minor Units: eine 3-Stellen-Währung skaliert auf drei Nachkommastellen ---------------
// BHD hat 3 Nachkommastellen: „1,234 BHD" sind 1234 Minor Units (nicht 123 400 wie unter der
// alten Skala-2-Annahme). Dieselbe Bridge, ein anderer ISO-Exponent — die Skala folgt der Währung.

#[test]
fn ein_bhd_betrag_skaliert_auf_drei_nachkommastellen() {
    let mut bhd = gültiger_rohsatz();
    bhd.amount = feld("+1,234");
    bhd.currency = feld("BHD");
    let api = ReadTestApi::new().with_line(bhd);

    api.run(QUELLE).expect("gültige BHD-Zeile");

    // 1,234 BHD = 1234 Minor Units (Skala 3), nicht 123 400 wie unter fester Skala 2.
    let lines = api.stored_lines();
    assert!(
        matches!(lines.as_slice(),
            [line] if line.amount_minor_units == 1234 && line.currency == "BHD"),
        "BHD skaliert auf 3 Nachkommastellen: {lines:?}"
    );
}

// --- Deutung: eine unbekannte Währung scheitert terminal ---------------------------------
// „Kennt die Domäne diese Währung?" ist eine Domänen-Deutungsentscheidung (`Currencies::resolve`),
// kein Präsenz-/Format-Check: die Zelle ist da, aber nicht deutbar → terminaler `CurrencyUnknown`.

#[test]
fn eine_unbekannte_währung_scheitert_terminal() {
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.currency = feld("XYZ");
    let api = ReadTestApi::new().with_line(rohsatz);

    let ergebnis = api.run(QUELLE);

    assert!(
        ergebnis.is_err(),
        "erwartet terminaler Fehler bei unbekannter Währung: {ergebnis:?}"
    );
    assert!(api.stored_lines().is_empty());
}

// --- Ablage: eine gescheiterte Ablage scheitert terminal ---------------------------------
// Die Ablage (io-Zugriff, der die Identität vergibt) ist fehlbar: scheitert sie, gibt der
// Handler seinen nackten terminalen `ReadError` zurück; die durchgereichte io-Diagnose bleibt im
// Grund. Weil die Veröffentlichung erst nach der Ablage kommt, reist nichts stromabwärts.

#[test]
fn eine_gescheiterte_ablage_scheitert_terminal() {
    let diagnose = "SQLite: disk I/O error";
    let api = ReadTestApi::new()
        .with_line(gültiger_rohsatz())
        .with_failing_store(diagnose);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(error) if error.to_string().contains(diagnose)),
        "erwartet terminaler Fehler mit durchgereichter io-Diagnose: {ergebnis:?}"
    );
    // Die Ablage scheitert vor der Veröffentlichung: nichts wird publiziert.
    assert!(api.published_message_types().is_empty());
}

// --- Veröffentlichung: eine gescheiterte Veröffentlichung scheitert terminal -------------
// Auch das Einreihen des „Eingelesen"-Ereignisses ist fehlbar und terminal: der Handler steigt
// mit seinem nackten `ReadError` aus, dessen Grund die durchgereichte io-Diagnose trägt.

#[test]
fn eine_gescheiterte_veröffentlichung_scheitert_terminal() {
    let diagnose = "Outbox: enqueue failed";
    let api = ReadTestApi::new()
        .with_line(gültiger_rohsatz())
        .with_failing_publish(diagnose);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(error) if error.to_string().contains(diagnose)),
        "erwartet terminaler Fehler mit durchgereichter io-Diagnose: {ergebnis:?}"
    );
}

// --- Zeilen-Schlüssel: ein neuer Import ergänzt (ADR-0016) -------------------------------
// Ein Auszug, der zweimal eingelesen wird, ergibt denselben Bestand: eine Zeile, deren Schlüssel
// schon abgelegt ist, wird weder erneut angelegt noch erneut publiziert — sonst legte `obfuskieren`
// sie ein zweites Mal ab. Beobachtet an beiden Datenrückgaben der TestApi.

#[test]
fn ein_zweiter_import_derselben_zeile_legt_nichts_an_und_publiziert_nichts() {
    let api = ReadTestApi::new().with_line(gültiger_rohsatz());
    api.run(QUELLE).expect("erster Import");

    api.run(QUELLE).expect("zweiter Import desselben Auszugs");

    assert_eq!(api.stored_lines(), vec![erwartete_zeile()]);
    assert_eq!(api.published_message_types(), vec!["Ingested"]);
    assert_eq!(api.stored_ids(), api.published_partition_keys());
}

#[test]
fn ein_neuer_import_ergaenzt_nur_die_unbekannten_zeilen() {
    let api = ReadTestApi::new().with_line(gültiger_rohsatz());
    api.run(QUELLE).expect("erster Import");
    // Der nächste Auszug überlappt: er trägt die schon bekannte Zeile **und** eine neue.
    let api = api.with_line(rohsatz_mit_schlüssel("zeile-2"));

    api.run(QUELLE).expect("überlappender Import");

    assert_eq!(
        api.stored_lines().len(),
        2,
        "nur die neue Zeile kommt hinzu"
    );
    assert_eq!(api.published_message_types(), vec!["Ingested"; 2]);
    assert_eq!(api.stored_ids(), api.published_partition_keys());
}

#[test]
fn zwei_zeilen_mit_demselben_schluessel_scheitern_terminal_redigiert() {
    // Die Quelle sagt eindeutige Schlüssel je Batch zu; ein Doppel hieße, eine echte Buchung still
    // als „schon bekannt" zu verwerfen. Der Grund nennt den Schlüssel nicht.
    let api = ReadTestApi::new()
        .with_line(gültiger_rohsatz())
        .with_line(gültiger_rohsatz());

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(error)
            if error.to_string().contains("row key") && !error.to_string().contains("zeile-1")),
        "erwartet redigierter Schlüssel-Doppel-Fehler: {ergebnis:?}"
    );
    assert!(api.stored_lines().is_empty());
    assert!(api.published_message_types().is_empty());
}

#[test]
fn eine_zeile_ohne_schluessel_scheitert_wie_ein_fehlendes_pflichtfeld() {
    let mut ohne_schlüssel = gültiger_rohsatz();
    ohne_schlüssel.row_key = None;
    let api = ReadTestApi::new().with_line(ohne_schlüssel);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(error) if error.to_string().contains("row_key")),
        "erwartet redigierter Pflichtfeld-Fehler: {ergebnis:?}"
    );
    assert!(api.stored_lines().is_empty());
}

// --- Vorgemerktes: überspringen, vor der Pflichtfeld-Prüfung (ADR-0016) ------------------
// Eine vorgemerkte Zeile hat eine leere Valuta — ein Pflichtfeld. Geprüft, ließe sie den ganzen
// Auszug scheitern; sie ist aber noch keine Buchung. Die ACL verwirft sie darum vor jeder Prüfung:
// der Import läuft durch, die Zeile zählt nicht.

#[test]
fn eine_vorgemerkte_zeile_bricht_den_import_nicht_ab_und_zaehlt_nicht() {
    let api = ReadTestApi::new()
        .with_line(vorgemerkter_rohsatz())
        .with_line(gültiger_rohsatz());

    api.run(QUELLE)
        .expect("eine vorgemerkte Zeile bricht den Import nicht ab");

    assert_eq!(api.stored_lines(), vec![erwartete_zeile()]);
    assert_eq!(api.published_message_types(), vec!["Ingested"]);
}

#[test]
fn ein_auszug_nur_aus_vorgemerktem_ergibt_nichts() {
    let api = ReadTestApi::new().with_line(vorgemerkter_rohsatz());

    api.run(QUELLE).expect("nichts Gebuchtes ist kein Fehler");

    assert!(api.stored_lines().is_empty());
    assert!(api.published_message_types().is_empty());
}
