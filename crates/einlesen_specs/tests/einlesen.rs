//! Black-Box-Spezifikation des einlesen-Slice — nur über die **eine** öffentliche
//! Adapter-Fläche (Fluent-`ReadTestApi`, das `RawRecord`-DTO und die aus `shared_kernel`
//! referenzierte Outbox-Ergebnisform `OutboxError`), nie über Interna. `run` fährt die
//! **echte** Bridge-Kette (Consume-Bridge → Kalender-/Geld-Bridge + `ReadHandler` →
//! Publisher-Bridge) gegen öffentliche Fakes der Abstractions.
//!
//! **Der maßgebliche Ablauf (Transaktionsgrenze):** das Kommando trägt bereits den ganzen
//! Batch (alle Rohzeilen); die Domäne deutet **pro Zeile** ein `LineAggregate` — scheitert
//! eine Zeile am Aggregat, ist das **terminal**: der Handler steigt mit dem Fehler aus, der
//! über die Consume-Bridge zu `OutboxError::Warning` bubblet, und die io-Outbox dead-lettert
//! die Nachricht (der Slice dead-lettert nicht selbst). Die gültigen Aggregate werden erst
//! **nach** vollständigem Durchlauf (`collect`) publiziert und atomar committet (der Commit
//! gehört io).
//!
//! **Warum es KEIN gemischtes „einige verworfen, andere laufen durch"-Szenario gibt**
//! (bewusst bestätigt, nicht getestet): Das ist kein konstruierbarer Zustand. Es ist
//! **alles-oder-nichts** — die erste terminal ungültige Zeile lässt den ganzen Lauf aussteigen
//! (nichts wird veröffentlicht), sonst laufen alle Zeilen durch.
//!
//! **Feld-Treue der `Line`-Projektion** (originalgetreue Übernahme **aller** Felder inkl.
//! Währung, Pflicht-/Optional-Verhalten) ist ein **Blackbox-Szenario**: `RawRecord` arrangieren,
//! `run` **einmal real** fahren, die gestagete `Line` am Speicher-Seam (`stored_lines`)
//! assertieren — kein Unit-Test der Projektion, sondern beobachtetes Pipeline-Verhalten. Das
//! ausgehende DTO wird am **konkreten** Seam-Typ beobachtet, nicht per generischem Serialisieren
//! (der `OutboxPublisher::publish`-Seam trägt bewusst keine `Serialize`-Schranke).
//!
//! **Was hier sehr wohl abgelehnt wird — und warum das Slice-Verhalten ist:** die *Präsenz*
//! eines Pflichtfelds erzwingt der **Adapter** (ACL, „außen → innen": `RawRecord` → `RawLine`);
//! fehlt eins, dead-lettert der **ganze** Batch redigiert (nur Feldname, nie ein Rohwert). Das
//! ist beobachtbares Slice-Verhalten, kein io-Detail — darum prüfen Specs es. Ebenso ist
//! „unbekannte Währung" eine **Domänen**-Deutungsentscheidung (`Currency::from_code`), kein
//! Format-/Präsenz-Check. Weiterhin **nicht** getestet wird reine io-Mechanik (Datei lesen,
//! echtes Dekodieren von deutschem Datum/Zahlenformat) — die liegt hinter den Seams.
//!
//! **`RawRecord` trägt keine Test-Ergonomie** (bewusst): die Fixture unten ist spec-lokal, nicht
//! Teil der Adapter-API.

use einlesen_adapter::{CalendarDay, Currency, Direction, Line, RawRecord, ReadTestApi};
use shared_kernel::OutboxError;

/// Die Quelle, aus der eine `ReadTransactions`-Nachricht ihren Batch bezieht (Aggregat-/
/// Partition-Identität).
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

    let ergebnis = api.run(QUELLE);

    assert_eq!(ergebnis, Ok(()));
    assert_eq!(api.published_message_types(), vec!["Ingested"]);
    // Genau **ein** Zeiger, und er deckt sich mit dem Speicher-Schlüssel (Tabelle = Wahrheit,
    // Message = Pointer): die Nachricht trägt nur die vergebene Aggregat-ID, unter der die Roh-Zeile
    // liegt. Ohne Kopplung an die interne Fake-Nummerierung.
    assert_eq!(api.published_partition_keys().len(), 1);
    assert_eq!(api.stored_ids(), api.published_partition_keys());
}

// --- Fan-out: pro Zeile genau ein Command (kein Batch stromabwärts) --------------

#[test]
fn mehrere_zeilen_fächern_zu_je_einem_command_auf() {
    let mut erste = gültiger_rohsatz();
    erste.recipient_name = feld("EDEKA");
    let mut zweite = gültiger_rohsatz();
    zweite.recipient_name = feld("REWE");
    let mut dritte = gültiger_rohsatz();
    dritte.recipient_name = feld("ALDI");

    let api = ReadTestApi::new()
        .with_line(erste)
        .with_line(zweite)
        .with_line(dritte);

    api.run(QUELLE).expect("gültige Zeilen");

    assert_eq!(
        api.published_message_types(),
        vec!["Ingested", "Ingested", "Ingested"]
    );
    // Genau **drei** Zeiger, und jeder mit einer **eigenen**, verschiedenen Aggregat-ID (kein
    // geteilter Schlüssel mehr) — ohne Kopplung an die interne Fake-Nummerierung.
    let zeiger = api.published_partition_keys();
    assert_eq!(zeiger.len(), 3);
    assert_eq!(
        zeiger
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3,
        "die drei Zeiger müssen verschieden sein"
    );
    // … und jeder Zeiger zeigt auf die unter genau dieser ID abgelegte Roh-Zeile (Message = Pointer).
    assert_eq!(api.stored_ids(), zeiger);
    // Aggregat **und** Ereignis werden atomar gestaged (ADR-0008): je gültiger Zeile liegt ihr
    // Auftragskonto am Speicher-Seam — in Lesereihenfolge, nicht nur der Fan-out stromabwärts.
    assert_eq!(
        api.stored_ordering_accounts(),
        vec![
            "DE89370400440532013000",
            "DE89370400440532013000",
            "DE89370400440532013000"
        ]
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

// --- Redaktion: der Dead-Letter-Grund spiegelt keinen Rohwert ----------------------
// Scheitert eine Zeile terminal, bubblet der Fehler zu `OutboxError::Warning`, dessen Grund
// io als Dead-Letter persistiert. Hier zählt **das Verhalten der Domäne**, nicht die
// io-seitige Format-/Vollständigkeitsprüfung (die die Domäne fordert, nicht selbst leistet):
// der Grund darf den Rohwert nie spiegeln.

#[test]
fn der_terminale_fehlergrund_spiegelt_den_rohwert_nicht() {
    // Spaltenverschub: eine IBAN landet im Betragsfeld. Der terminale Grund wird als
    // Dead-Letter-reason der Outbox persistiert — er darf den Rohwert nicht spiegeln. Die
    // Redaktion ist Domänen-Invariante; hier prüfen wir, dass sie bis in die Outbox-Meldung trägt.
    let leck = "DE89370400440532013000";
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.amount = feld(&format!("-{leck}"));
    let api = ReadTestApi::new().with_line(rohsatz);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason }) if !reason.contains(leck)),
        "der Dead-Letter-Grund leakt den Rohwert: {ergebnis:?}"
    );
}

// --- Präsenz: ein fehlendes Pflichtfeld lettert den GANZEN Batch ins Dead-Letter ---------
// Die Präsenz eines Pflichtfelds erzwingt der Adapter (ACL). Fehlt eins irgendwo, scheitert die
// „außen → innen"-Abbildung des Batches, bevor der Handler eine Zeile deutet — batch-atomar:
// nichts wird gestaged/veröffentlicht, auch nicht die gültige Zeile daneben. Der Grund ist
// redigiert (nur der Feldname).

#[test]
fn ein_fehlendes_pflichtfeld_lettert_den_ganzen_batch() {
    let gültige = gültiger_rohsatz();
    let mut ohne_betrag = gültiger_rohsatz();
    ohne_betrag.amount = None;
    let api = ReadTestApi::new().with_line(gültige).with_line(ohne_betrag);

    let ergebnis = api.run(QUELLE);

    // Dead-Letter, dessen Grund nur den Feldnamen benennt (kein Rohwert).
    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason }) if reason.contains("amount")),
        "erwartet redigierter Dead-Letter (nur Feldname): {ergebnis:?}"
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

// --- Deutung: eine unbekannte Währung lettert ins Dead-Letter ----------------------------
// „Kennt die Domäne diese Währung?" ist eine Domänen-Deutungsentscheidung (`Currency::from_code`),
// kein Präsenz-/Format-Check: die Zelle ist da, aber nicht deutbar → terminaler `CurrencyUnknown`.

#[test]
fn eine_unbekannte_währung_lettert_ins_dead_letter() {
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.currency = feld("XYZ");
    let api = ReadTestApi::new().with_line(rohsatz);

    let ergebnis = api.run(QUELLE);

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { .. })),
        "erwartet Dead-Letter bei unbekannter Währung: {ergebnis:?}"
    );
    assert!(api.stored_lines().is_empty());
}
