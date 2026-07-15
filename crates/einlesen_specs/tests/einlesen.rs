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
//! Nachricht. Die Domäne deutet **pro Zeile** ein `LineAggregate` — scheitert die Beschaffung
//! oder eine Zeile am Aggregat, ist das **terminal**: der Handler steigt mit dem `ReadError`
//! aus (den `io` dead-lettert, der Slice dead-lettert nicht selbst). Die gültigen Aggregate
//! werden erst **nach** vollständigem Durchlauf (`collect`) gestaged; den atomaren Commit
//! besitzt `io`.
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
//! fehlt eins, scheitert der **ganze** Batch redigiert (nur Feldname, nie ein Rohwert). Das
//! ist beobachtbares Slice-Verhalten, kein io-Detail — darum prüfen Specs es. Ebenso sind die
//! Abbildung „Quelle nicht beschaffbar → terminaler Fehler mit durchgereichter io-Diagnose"
//! (Bridge-Verhalten, kein io-Format-Detail) und „unbekannte Währung" (eine
//! **Domänen**-Deutungsentscheidung, `Currency::from_code`) Slice-Verhalten. Weiterhin
//! **nicht** getestet wird reine io-Mechanik (Datei lesen, echtes Dekodieren von deutschem
//! Datum/Zahlenformat) — die liegt hinter den Seams.
//!
//! **`RawRecord` trägt keine Test-Ergonomie** (bewusst): die Fixture unten ist spec-lokal, nicht
//! Teil der Adapter-API.

use einlesen_adapter::{CalendarDay, Currency, Direction, Line, RawRecord, ReadTestApi};

/// Die Quelle eines Einlese-Laufs — seit Issue 0018 reiner Routing-Kontext: sie reist nur
/// noch als `partition_key` des Envelopes (die Payload ist feldlos), der Batch kommt über
/// den Quell-Seam.
const QUELLE: &str = "auszug-juni";

fn feld(wert: &str) -> Option<String> {
    Some(wert.to_string())
}

/// Ein vollständig gültiger Rohsatz — spec-lokale Fixture; Feld-Overrides passieren direkt
/// (die Felder sind `pub`, kein Fluent-Bauer nötig).
fn gültiger_rohsatz() -> RawRecord {
    RawRecord {
        ordering_account: feld("DE89370400440532013000"),
        booking_day: feld("13.06.2024"),
        value_date: feld("14.06.2024"),
        amount: feld("-50,00"),
        currency: feld("EUR"),
        recipient_name: feld("EDEKA SAGT DANKE"),
        creditor_id: feld("DE98ZZZ09999999999"),
        counter_account: feld("DE11520513735120710131"),
        purpose: feld("Einkauf"),
        booking_text: feld("LASTSCHRIFT"),
    }
}

/// Die erwartete Projektion des [`gültiger_rohsatz`] auf das ausgehende `Line`-DTO — spec-lokale
/// Fixture, damit die Feld-Treue-Specs eine **ganze** `Line` vergleichen (`assert_eq!`), statt
/// Feld für Feld zu assertieren. Die `Line`-Felder sind `pub`, ein Struct-Literal genügt.
fn erwartete_zeile() -> Line {
    Line {
        ordering_account: "DE89370400440532013000".to_string(),
        counter_account: Some("DE11520513735120710131".to_string()),
        amount_cent: 5_000,
        currency: Currency::Euro,
        direction: Direction::Outgoing,
        recipient_name: Some("EDEKA SAGT DANKE".to_string()),
        creditor_id: Some("DE98ZZZ09999999999".to_string()),
        booking_day: CalendarDay {
            year: 2024,
            month: 6,
            day: 13,
        },
        value_date: CalendarDay {
            year: 2024,
            month: 6,
            day: 14,
        },
        purpose: Some("Einkauf".to_string()),
        booking_text: Some("LASTSCHRIFT".to_string()),
    }
}

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
            let mut rohsatz = gültiger_rohsatz();
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
// jedes projizierte Feld — inkl. der **Währung** neben dem Cent-Betrag — kommt originalgetreu an.

#[test]
fn die_gestagte_zeile_übernimmt_alle_felder_originalgetreu() {
    let api = ReadTestApi::new().with_line(gültiger_rohsatz());

    api.run(QUELLE).expect("gültige Zeile");

    // Die gestagete `Line` als Ganzes gegen die erwartete Projektion — ein `assert_eq!` statt
    // Feld-für-Feld, inkl. der Währung neben dem Cent-Betrag.
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
// `Display` io als Dead-Letter-Grund spiegelt. Hier zählt **das Verhalten der Domäne**, nicht die
// io-seitige Format-/Vollständigkeitsprüfung (die die Domäne fordert, nicht selbst leistet):
// der Grund darf den Rohwert nie spiegeln.

#[test]
fn der_terminale_fehlergrund_spiegelt_den_rohwert_nicht() {
    // Spaltenverschub: eine IBAN landet im Betragsfeld. Die `Display` des terminalen `ReadError`
    // wird io als Dead-Letter-reason persistieren — sie darf den Rohwert nicht spiegeln. Die
    // Redaktion ist Domänen-Invariante; hier prüfen wir, dass sie bis in die Fehlermeldung trägt.
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
fn ein_fehlendes_pflichtfeld_lettert_den_ganzen_batch() {
    let gültige = gültiger_rohsatz();
    let mut ohne_betrag = gültiger_rohsatz();
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
// Nichts wird gestaged, nichts publiziert.

#[test]
fn eine_nicht_beschaffbare_quelle_lettert_ins_dead_letter() {
    let diagnose = "CSV-Datei nicht lesbar: auszug-juni.csv (permission denied)";
    let api = ReadTestApi::new().with_unavailable_source(diagnose);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(error) if error.to_string().contains(diagnose)),
        "erwartet terminaler Fehler mit durchgereichter io-Diagnose: {ergebnis:?}"
    );
    // Terminal und leer: nichts wird gestaged oder veröffentlicht.
    assert!(api.stored_lines().is_empty());
    assert!(api.published_message_types().is_empty());
}

// --- Deutung: eine unbekannte Währung scheitert terminal ---------------------------------
// „Kennt die Domäne diese Währung?" ist eine Domänen-Deutungsentscheidung (`Currency::from_code`),
// kein Präsenz-/Format-Check: die Zelle ist da, aber nicht deutbar → terminaler `CurrencyUnknown`.

#[test]
fn eine_unbekannte_währung_lettert_ins_dead_letter() {
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
