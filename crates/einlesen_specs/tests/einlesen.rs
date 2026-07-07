//! Black-Box-Spezifikation des einlesen-Slice — nur über die **eine** öffentliche
//! Adapter-Fläche (Fluent-`EinlesenTestApi`, das `Rohsatz`-DTO und die aus `shared_kernel`
//! referenzierte Outbox-Ergebnisform `OutboxError`), nie über Interna. `ausführen` fährt die
//! **echte** Bridge-Kette (Consume-Bridge → Kalender-Bridge + `EinleseHandler` →
//! Publisher-Bridge) gegen öffentliche Fakes der Abstractions.
//!
//! **Der maßgebliche Ablauf (Transaktionsgrenze):** das Kommando trägt bereits den ganzen
//! Batch (alle Rohzeilen); die Domäne deutet **pro Zeile** ein `ZeilenAggregat` — scheitert
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
//! **Feld-Treue der `Zeile`-Projektion** (originalgetreue Übernahme aller Felder,
//! Pflicht-/Optional-Verhalten) ist **kein** Blackbox-Szenario mehr: sie ist ein direkter
//! Unit-Test von `Zeile::aus_aggregat` im Adapter (`crates/einlesen_adapter/src/zeile.rs`) —
//! reine Mapping-Algorithmik, kein Pipeline-Verhalten.
//!
//! **`Rohsatz` trägt keine Test-Ergonomie** (bewusst): die Fixture unten ist spec-lokal, nicht
//! Teil der Adapter-API.

use einlesen_adapter::{EinlesenTestApi, Rohsatz};
use shared_kernel::OutboxError;

/// Die Quelle, aus der eine `UmsätzeEinlesen`-Nachricht ihren Batch bezieht (Aggregat-/
/// Partition-Identität).
const QUELLE: &str = "auszug-juni";

fn feld(wert: &str) -> Option<String> {
    Some(wert.to_string())
}

/// Ein vollständig gültiger Rohsatz — spec-lokale Fixture; Feld-Overrides passieren direkt
/// (die Felder sind `pub`, kein Fluent-Bauer nötig).
fn gültiger_rohsatz() -> Rohsatz {
    Rohsatz {
        auftragskonto: feld("DE89370400440532013000"),
        buchungstag: feld("13.06.2024"),
        valuta: feld("14.06.2024"),
        betrag: feld("-50,00"),
        währung: feld("EUR"),
        empfänger_name: feld("EDEKA SAGT DANKE"),
        gläubiger_id: feld("DE98ZZZ09999999999"),
        gegenkonto: feld("DE11520513735120710131"),
        verwendungszweck: feld("Einkauf"),
        buchungstext: feld("LASTSCHRIFT"),
    }
}

// --- Eine gültige Zeile: genau ein Aggregat + genau ein Command -------------------

#[test]
fn eine_gültige_zeile_wird_zu_einem_eingelesen_command() {
    let api = EinlesenTestApi::new().mit_zeile(gültiger_rohsatz());

    let ergebnis = api.ausführen(QUELLE);

    assert_eq!(ergebnis, Ok(()));
    assert_eq!(api.veröffentlichte_message_types(), vec!["Eingelesen"]);
    assert_eq!(
        api.veröffentlichte_partition_keys(),
        vec!["DE89370400440532013000"]
    );
}

// --- Fan-out: pro Zeile genau ein Command (kein Batch stromabwärts) --------------

#[test]
fn mehrere_zeilen_fächern_zu_je_einem_command_auf() {
    let mut erste = gültiger_rohsatz();
    erste.empfänger_name = feld("EDEKA");
    let mut zweite = gültiger_rohsatz();
    zweite.empfänger_name = feld("REWE");
    let mut dritte = gültiger_rohsatz();
    dritte.empfänger_name = feld("ALDI");

    let api = EinlesenTestApi::new()
        .mit_zeile(erste)
        .mit_zeile(zweite)
        .mit_zeile(dritte);

    api.ausführen(QUELLE).expect("gültige Zeilen");

    assert_eq!(
        api.veröffentlichte_message_types(),
        vec!["Eingelesen", "Eingelesen", "Eingelesen"]
    );
    assert_eq!(
        api.veröffentlichte_partition_keys(),
        vec![
            "DE89370400440532013000",
            "DE89370400440532013000",
            "DE89370400440532013000"
        ]
    );
}

// --- Eine ungültige Zeile: terminaler Fehler bubblet auf (io dead-lettert) ---------
// Die Zeile kam über io (strukturell valide), ist aber am Aggregat kaputt: das ist terminal und
// muss reportet werden — der Handler steigt mit dem Fehler aus, er wird zu `OutboxError::Warning`
// (Dead-Letter). Nichts wird veröffentlicht (die Transaktion umschließt erst die vollständig
// gültigen Aggregate).

#[test]
fn eine_zeile_ohne_betrag_scheitert_terminal() {
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.betrag = None;
    let api = EinlesenTestApi::new().mit_zeile(rohsatz);

    let ergebnis = api.ausführen(QUELLE);

    assert!(
        matches!(ergebnis, Err(OutboxError::Warning { .. })),
        "ein am Aggregat ungültiges Feld muss terminal (Dead-Letter) scheitern, war: {ergebnis:?}"
    );
    assert!(api.veröffentlichte_message_types().is_empty());
    assert!(api.veröffentlichte_partition_keys().is_empty());
}

#[test]
fn eine_zeile_mit_ungültigem_datum_scheitert_terminal() {
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.valuta = feld("kein-datum");
    let api = EinlesenTestApi::new().mit_zeile(rohsatz);

    let ergebnis = api.ausführen(QUELLE);

    assert!(
        matches!(ergebnis, Err(OutboxError::Warning { .. })),
        "ein ungültiges Datum muss terminal scheitern, war: {ergebnis:?}"
    );
    assert!(api.veröffentlichte_message_types().is_empty());
    assert!(api.veröffentlichte_partition_keys().is_empty());
}

#[test]
fn der_terminale_fehlergrund_spiegelt_den_rohwert_nicht() {
    // Spaltenverschub: eine IBAN landet im Betragsfeld. Der terminale Grund wird als
    // Dead-Letter-reason der Outbox persistiert — er darf den Rohwert nicht spiegeln. Die
    // Redaktion ist Domänen-Invariante; hier prüfen wir, dass sie bis in die Outbox-Meldung trägt.
    let leck = "DE89370400440532013000";
    let mut rohsatz = gültiger_rohsatz();
    rohsatz.betrag = feld(&format!("-{leck}"));
    let api = EinlesenTestApi::new().mit_zeile(rohsatz);

    let ergebnis = api.ausführen(QUELLE);

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason }) if !reason.contains(leck)),
        "der Dead-Letter-Grund leakt den Rohwert: {ergebnis:?}"
    );
}

#[test]
fn ungueltiger_payload_wird_zu_critical() {
    // Slice-spezifische Blackbox: ein kaputter JSON-Payload einer
    // `UmsätzeEinlesen`-Nachricht darf nicht als Warning/Retry laufen, sondern
    // muss als Critical enden.
    let api = EinlesenTestApi::new();
    let ergebnis = api.ausführen_mit_payload(QUELLE, "{ kaputt-json".to_string());

    assert!(
        matches!(&ergebnis, Err(OutboxError::Critical { source, .. }) if source == "UmsätzeEinlesen"),
        "ungültiger Payload muss als Critical enden, war: {ergebnis:?}"
    );
}
