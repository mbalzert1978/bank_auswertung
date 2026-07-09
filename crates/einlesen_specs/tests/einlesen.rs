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
//! **Feld-Treue der `Line`-Projektion** (originalgetreue Übernahme aller Felder,
//! Pflicht-/Optional-Verhalten) ist **kein** Blackbox-Szenario mehr: sie ist ein direkter
//! Unit-Test von `Line::from_aggregate` im Adapter (`crates/einlesen_adapter/src/dto/line.rs`) —
//! reine Mapping-Algorithmik, kein Pipeline-Verhalten.
//!
//! **`RawRecord` trägt keine Test-Ergonomie** (bewusst): die Fixture unten ist spec-lokal, nicht
//! Teil der Adapter-API.

use einlesen_adapter::{RawRecord, ReadTestApi};
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

// --- Eine gültige Zeile: genau ein Aggregat + genau ein Command -------------------

#[test]
fn eine_gültige_zeile_wird_zu_einem_eingelesen_command() {
    let api = ReadTestApi::new().with_line(gültiger_rohsatz());

    let ergebnis = api.run(QUELLE);

    assert_eq!(ergebnis, Ok(()));
    assert_eq!(api.published_message_types(), vec!["Ingested"]);
    assert_eq!(
        api.published_partition_keys(),
        vec!["DE89370400440532013000"]
    );
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
    assert_eq!(
        api.published_partition_keys(),
        vec![
            "DE89370400440532013000",
            "DE89370400440532013000",
            "DE89370400440532013000"
        ]
    );
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
