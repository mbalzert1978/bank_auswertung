//! Black-Box-Spezifikation des einlesen-Slice — nur über die **eine** öffentliche
//! Adapter-Fläche (Fluent-`EinlesenTestApi`, der `Rohsatz`-Arrange-Builder, das `Zeile`-DTO
//! und die aus `shared_kernel` referenzierte Outbox-Ergebnisform `OutboxError`), nie über
//! Interna. `ausführen` fährt die **echte** Bridge-Kette (Consume-Bridge → `EinleseHandler`
//! → Port-/Publisher-Bridges) gegen öffentliche Fakes.
//!
//! **Der maßgebliche Ablauf (Transaktionsgrenze):** (a) alle Rohzeilen werden aus der Quelle
//! gelesen — ein transienter Lesefehler ist retryable und wird zu `OutboxError::Retry`; (b)
//! die Domäne deutet **pro Zeile** ein `ZeilenAggregat` — scheitert eine (von io gelesene,
//! strukturell valide) Zeile am Aggregat, ist das **terminal**: der Handler steigt mit dem
//! Fehler aus, der über die Consume-Bridge zu `OutboxError::Warning` bubblet, und die io-Outbox
//! dead-lettert die Nachricht (der Slice dead-lettert nicht selbst); (c–f) die gültigen
//! Aggregate werden **gemeinsam mit je genau einem** ausgehenden `Eingelesen`-Command gestaget
//! und atomar committet (der Commit gehört io).
//!
//! **Warum es KEIN gemischtes „einige verworfen, andere laufen durch"-Szenario gibt**
//! (bewusst bestätigt, nicht getestet): Das ist kein konstruierbarer Zustand. Es ist
//! **alles-oder-nichts** — die erste terminal ungültige Zeile lässt den ganzen Lauf aussteigen
//! (nichts wird gestaget/veröffentlicht), sonst laufen alle Zeilen durch. Die Invariante lautet
//! **Aggregat ⇔ Command atomar** — ein Aggregat ohne (oder mit fehlendem) Command kann nie
//! entstehen. Gültige und ungültige Zeilen werden darum je **isoliert** spezifiziert; die
//! Zusammensetzung ist die triviale Superposition dieser Einheiten, kein eigener Zustand.

use einlesen_adapter::{EinlesenTestApi, Richtung, Rohsatz};
use shared_kernel::OutboxError;

/// Die Quelle, aus der eine `UmsätzeEinlesen`-Nachricht ihre Rohzeilen bezieht (im Fake
/// vorab per `mit_zeile` bestückt; die Kennung adressiert später die echte io-Quelle).
const QUELLE: &str = "auszug-juni";

// --- Eine gültige Zeile: genau ein Aggregat + genau ein Command -------------------

#[test]
fn eine_gültige_zeile_wird_zu_einem_eingelesen_command() {
    let mut api = EinlesenTestApi::new().mit_zeile(Rohsatz::gültig());

    let ergebnis = api.ausführen(QUELLE);

    assert_eq!(ergebnis, Ok(()));
    assert_eq!(api.gestagte_zeilen().len(), 1);
    assert_eq!(api.veröffentlichte_eingelesen().len(), 1);
}

#[test]
fn die_felder_der_zeile_werden_originalgetreu_übernommen() {
    let mut api = EinlesenTestApi::new().mit_zeile(Rohsatz::gültig());

    api.ausführen(QUELLE).expect("gültige Zeile");

    let zeilen = api.gestagte_zeilen();
    let zeile = zeilen.first().expect("eine gestagte Zeile");
    assert_eq!(zeile.auftragskonto, "DE89370400440532013000");
    assert_eq!(zeile.gegenkonto.as_deref(), Some("DE11520513735120710131"));
    assert_eq!(zeile.betrag_cent, 5_000);
    assert_eq!(zeile.richtung, Richtung::Ausgang);
    assert_eq!(zeile.empfänger_name, "EDEKA SAGT DANKE");
    assert_eq!(zeile.gläubiger_id.as_deref(), Some("DE98ZZZ09999999999"));
    assert_eq!(zeile.buchungstag.tag(), 13);
    assert_eq!(zeile.valuta.tag(), 14);
    assert_eq!(zeile.verwendungszweck.as_deref(), Some("Einkauf"));
    assert_eq!(zeile.buchungstext.as_deref(), Some("LASTSCHRIFT"));
}

#[test]
fn fehlende_optionalfelder_ergeben_eine_zeile_ohne_diese_felder() {
    let mut api = EinlesenTestApi::new().mit_zeile(
        Rohsatz::gültig()
            .ohne_gegenkonto()
            .ohne_verwendungszweck()
            .ohne_buchungstext(),
    );

    api.ausführen(QUELLE).expect("Pflichtfelder gesetzt");

    let zeilen = api.gestagte_zeilen();
    let zeile = zeilen.first().expect("eine gestagte Zeile");
    assert!(zeile.gegenkonto.is_none());
    assert!(zeile.verwendungszweck.is_none());
    assert!(zeile.buchungstext.is_none());
}

// --- Fan-out: pro Zeile genau ein Aggregat+Command-Paar (kein Batch) --------------

#[test]
fn mehrere_zeilen_fächern_zu_je_einem_aggregat_command_paar_auf() {
    let mut api = EinlesenTestApi::new()
        .mit_zeile(Rohsatz::gültig().mit_empfänger_name("EDEKA"))
        .mit_zeile(Rohsatz::gültig().mit_empfänger_name("REWE"))
        .mit_zeile(Rohsatz::gültig().mit_empfänger_name("ALDI"));

    api.ausführen(QUELLE).expect("gültige Zeilen");

    // Fan-out: drei Zeilen → drei Aggregate → drei Commands; die Zähl-Gleichheit
    // beobachtet die Invariante „Aggregat ⇔ Command atomar".
    assert_eq!(api.gestagte_zeilen().len(), 3);
    assert_eq!(api.veröffentlichte_eingelesen().len(), 3);
}

// --- Eine ungültige Zeile: terminaler Fehler bubblet auf (io dead-lettert) ---------
// Die Zeile kam über io (strukturell valide), ist aber am Aggregat kaputt: das ist terminal und
// muss reportet werden — der Handler steigt mit dem Fehler aus, er wird zu `OutboxError::Warning`
// (Dead-Letter). Nichts wird gestaget/veröffentlicht (die Transaktion umschließt erst die
// vollständig gültigen Aggregate).

#[test]
fn eine_zeile_ohne_betrag_scheitert_terminal() {
    let mut api = EinlesenTestApi::new().mit_zeile(Rohsatz::gültig().ohne_betrag());

    let ergebnis = api.ausführen(QUELLE);

    assert!(
        matches!(ergebnis, Err(OutboxError::Warning { .. })),
        "ein am Aggregat ungültiges Feld muss terminal (Dead-Letter) scheitern, war: {ergebnis:?}"
    );
    assert!(api.gestagte_zeilen().is_empty());
    assert!(api.veröffentlichte_eingelesen().is_empty());
}

#[test]
fn eine_zeile_mit_ungültigem_datum_scheitert_terminal() {
    let mut api = EinlesenTestApi::new().mit_zeile(Rohsatz::gültig().mit_valuta("kein-datum"));

    let ergebnis = api.ausführen(QUELLE);

    assert!(
        matches!(ergebnis, Err(OutboxError::Warning { .. })),
        "ein ungültiges Datum muss terminal scheitern, war: {ergebnis:?}"
    );
    assert!(api.gestagte_zeilen().is_empty());
    assert!(api.veröffentlichte_eingelesen().is_empty());
}

#[test]
fn der_terminale_fehlergrund_spiegelt_den_rohwert_nicht() {
    // Spaltenverschub: eine IBAN landet im Betragsfeld. Der terminale Grund wird als
    // Dead-Letter-reason der Outbox persistiert — er darf den Rohwert nicht spiegeln. Die
    // Redaktion ist Domänen-Invariante; hier prüfen wir, dass sie bis in die Outbox-Meldung trägt.
    let leck = "DE89370400440532013000";
    let mut api =
        EinlesenTestApi::new().mit_zeile(Rohsatz::gültig().mit_betrag(&format!("-{leck}")));

    let ergebnis = api.ausführen(QUELLE);

    assert!(
        matches!(&ergebnis, Err(OutboxError::Warning { reason }) if !reason.contains(leck)),
        "der Dead-Letter-Grund leakt den Rohwert: {ergebnis:?}"
    );
}

// --- Retry: transienter Lesefehler der Quelle → nichts wird sichtbar --------------

#[test]
fn ein_lesefehler_der_quelle_führt_zu_retry() {
    let mut api = EinlesenTestApi::new()
        .mit_zeile(Rohsatz::gültig())
        .mit_quellenfehler("Quelle kurzzeitig nicht lesbar");

    let ergebnis = api.ausführen(QUELLE);

    assert!(
        matches!(ergebnis, Err(OutboxError::Retry { .. })),
        "ein transienter Lesefehler muss Retry auslösen, war: {ergebnis:?}"
    );
    // Der Lesefehler fällt vor der Transaktion — nichts wird gestaget oder veröffentlicht.
    assert!(api.gestagte_zeilen().is_empty());
    assert!(api.veröffentlichte_eingelesen().is_empty());
}
