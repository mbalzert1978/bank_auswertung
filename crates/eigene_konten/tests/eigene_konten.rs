use eigene_konten::{
    ConfigSource, ConfigSourceError, EigeneKonten, Iban, IbanError, Kontozuordnung, LoadError,
    Rolle,
};

// --- Test-Adapter für den ConfigSource-Port -------------------------------

/// Datei-basierte ConfigSource. `std::fs` lebt hier im Testcode (am Rand) —
/// der Crate selbst bleibt fs-frei.
struct FileSource {
    path: std::path::PathBuf,
}

impl ConfigSource for FileSource {
    fn read(&self) -> Result<String, ConfigSourceError> {
        Ok(std::fs::read_to_string(&self.path)?)
    }
}

/// In-Memory-ConfigSource — belegt, dass der Loader nicht an eine Datei
/// gebunden ist.
struct StringSource(&'static str);

impl ConfigSource for StringSource {
    fn read(&self) -> Result<String, ConfigSourceError> {
        Ok(self.0.to_string())
    }
}

/// ConfigSource, die immer scheitert — für den Lesefehler-Pfad.
struct FailingSource;

impl ConfigSource for FailingSource {
    fn read(&self) -> Result<String, ConfigSourceError> {
        Err("Quelle kaputt".into())
    }
}

fn dummy_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eigene-konten.dummy.toml")
}

fn fixture_konten() -> EigeneKonten {
    EigeneKonten::from_source(&FileSource { path: dummy_path() }).expect("Dummy-Fixture lädt")
}

fn iban(input: &str) -> Iban {
    Iban::parse(input).expect("gültige IBAN")
}

// --- Nachschlagen über die Fixture (mehrere Stellen) ----------------------

#[test]
fn eigene_giro_iban_wird_als_eigen_mit_rolle_giro_erkannt() {
    assert_eq!(
        fixture_konten().lookup(&iban("DE20 1234 5678 0000 0011 11")),
        Kontozuordnung::Eigen(Rolle::Giro)
    );
}

#[test]
fn eigene_vorlauf_iban_wird_als_eigen_mit_rolle_vorlauf_erkannt() {
    assert_eq!(
        fixture_konten().lookup(&iban("DE45 1234 5678 0000 0044 44")),
        Kontozuordnung::Eigen(Rolle::Vorlauf)
    );
}

#[test]
fn fremde_iban_wird_als_fremd_erkannt() {
    assert_eq!(
        fixture_konten().lookup(&iban("DE90 8765 4321 0000 0099 99")),
        Kontozuordnung::Fremd
    );
}

// Eine-Quelle-Beleg: dieselbe Fixture-Instanz beantwortet Eigen/Fremd UND Rolle.
#[test]
fn dieselbe_config_beantwortet_eigen_fremd_und_rolle() {
    let konten = fixture_konten();

    assert_eq!(
        konten.lookup(&iban("DE69 1234 5678 0000 0033 33")),
        Kontozuordnung::Eigen(Rolle::Spar)
    );
    assert_eq!(
        konten.lookup(&iban("DE93 1234 5678 0000 0022 22")),
        Kontozuordnung::Eigen(Rolle::Haushalt)
    );
    assert_eq!(
        konten.lookup(&iban("DE90 8765 4321 0000 0099 99")),
        Kontozuordnung::Fremd
    );
}

#[test]
fn nachschlagen_ignoriert_schreibweise_und_leerzeichen() {
    assert_eq!(
        fixture_konten().lookup(&iban("de2012345678000000 1111")),
        Kontozuordnung::Eigen(Rolle::Giro)
    );
}

// --- Port-Entkopplung -----------------------------------------------------

#[test]
fn config_wird_auch_ohne_datei_geladen() {
    let konten = EigeneKonten::from_source(&StringSource(
        "[[konto]]\niban = \"DE20 1234 5678 0000 0011 11\"\nrolle = \"Giro\"\n",
    ))
    .expect("gültige In-Memory-Config");

    assert_eq!(
        konten.lookup(&iban("DE20 1234 5678 0000 0011 11")),
        Kontozuordnung::Eigen(Rolle::Giro)
    );
}

#[test]
fn lesefehler_der_quelle_wird_als_ladefehler_gemeldet() {
    assert!(matches!(
        EigeneKonten::from_source(&FailingSource),
        Err(LoadError::Source(_))
    ));
}

// --- Ablehnung statt stillem Überspringen (from_toml, kein File nötig) -----

#[test]
fn config_mit_unbekannter_rolle_wird_abgelehnt() {
    let toml = "[[konto]]\niban = \"DE20 1234 5678 0000 0011 11\"\nrolle = \"Tagesgeld\"\n";

    assert!(matches!(
        EigeneKonten::from_toml(toml),
        Err(LoadError::Toml(_))
    ));
}

#[test]
fn config_mit_ungültiger_iban_wird_abgelehnt() {
    // gültige Giro-IBAN mit verfälschter Prüfziffer (20 -> 21)
    let toml = "[[konto]]\niban = \"DE21 1234 5678 0000 0011 11\"\nrolle = \"Giro\"\n";

    assert!(matches!(
        EigeneKonten::from_toml(toml),
        Err(LoadError::Toml(_))
    ));
}

#[test]
fn config_mit_doppelter_iban_wird_abgelehnt() {
    let toml = "[[konto]]\niban = \"DE20 1234 5678 0000 0011 11\"\nrolle = \"Giro\"\n\n\
                [[konto]]\niban = \"DE20 1234 5678 0000 0011 11\"\nrolle = \"Spar\"\n";

    assert!(matches!(
        EigeneKonten::from_toml(toml),
        Err(LoadError::DuplicateIban(_))
    ));
}

// Eine leere Config ist eine Fehlkonfiguration, kein „alles fremd": die eine
// Quelle für Eigen/Fremd wäre leer. `konto` ist Pflichtfeld → laute Ablehnung.
#[test]
fn config_ganz_ohne_konto_eintrag_wird_abgelehnt() {
    assert!(matches!(
        EigeneKonten::from_toml(""),
        Err(LoadError::Toml(_))
    ));
}

// --- Iban::parse: jede Fehlervariante direkt ------------------------------

#[test]
fn zu_kurze_iban_wird_abgelehnt() {
    assert_eq!(Iban::parse("DE12"), Err(IbanError::Length(4)));
}

#[test]
fn iban_mit_falschem_aufbau_wird_abgelehnt() {
    // erste zwei Zeichen sind keine Buchstaben
    assert_eq!(Iban::parse("1234567890123456"), Err(IbanError::Structure));
}

#[test]
fn iban_mit_ungültigem_zeichen_wird_abgelehnt() {
    assert_eq!(
        Iban::parse("DE0012345678-01234"),
        Err(IbanError::InvalidChar)
    );
}

#[test]
fn iban_mit_falscher_prüfsumme_wird_abgelehnt() {
    assert_eq!(
        Iban::parse("DE21 1234 5678 0000 0011 11"),
        Err(IbanError::Checksum)
    );
}

#[test]
fn gültige_iban_wird_akzeptiert() {
    Iban::parse("DE20 1234 5678 0000 0011 11").expect("gültige IBAN");
}
