//! Die Mechanik des Export-Schreibers (Issue 0011): Rechte `0600`, Umbenennen statt Überschreiben,
//! eine alte Datei bleibt bei einem Fehler unberührt. Den **Inhalt** prüfen die Lauf-Tests
//! (`run/kategorien_tests.rs`) — eine Anfrage-Zeile kann nur die Domäne bauen.

use super::*;

/// Ein eigener, leerer Ordner im Temp-Verzeichnis, der nach dem Test verschwindet.
struct Ordner(PathBuf);

impl Ordner {
    fn neu(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dauer| dauer.as_nanos())
            .unwrap_or(0);
        let pfad = std::env::temp_dir().join(format!(
            "bank_auswertung_export_{tag}_{}_{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&pfad).expect("der Test-Ordner lässt sich anlegen");
        Self(pfad)
    }
}

impl Drop for Ordner {
    fn drop(&mut self) {
        let _aufraeumen: std::io::Result<()> = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn ohne_anfragen_entsteht_eine_gueltige_datei_mit_leerer_liste() {
    let ordner = Ordner::neu("leer");
    let ziel = ordner.0.join("anfrage.json");

    write(&ziel, &[]).expect("der Export gelingt");

    let inhalt: Value =
        serde_json::from_str(&fs::read_to_string(&ziel).expect("lesbar")).expect("gültiges JSON");
    assert_eq!(inhalt.get("format"), Some(&Value::from(FORMAT)));
    assert_eq!(inhalt.get("anfragen"), Some(&json!([])));
    // Nur die drei festen Schlüssel — nichts sonst steht oben.
    assert_eq!(inhalt.as_object().map(Map::len), Some(3));
    // … und in der Reihenfolge des Tickets: erst Format und Anleitung, dann die Anfragen.
    let text = render(&[]);
    let stelle = |schluessel: &str| text.find(schluessel);
    assert!(stelle("\"format\"") < stelle("\"anleitung\""));
    assert!(stelle("\"anleitung\"") < stelle("\"anfragen\""));
}

#[cfg(unix)]
#[test]
fn die_datei_ist_nur_fuer_den_nutzer_lesbar() {
    use std::os::unix::fs::PermissionsExt;
    let ordner = Ordner::neu("rechte");
    let ziel = ordner.0.join("anfrage.json");

    write(&ziel, &[]).expect("der Export gelingt");

    let rechte = fs::metadata(&ziel).expect("vorhanden").permissions().mode() & 0o777;
    assert_eq!(rechte, 0o600);
}

#[test]
fn eine_alte_datei_wird_ersetzt_und_keine_nachbar_datei_bleibt_liegen() {
    let ordner = Ordner::neu("ersetzen");
    let ziel = ordner.0.join("anfrage.json");
    fs::write(&ziel, "alt").expect("eine alte Datei");

    write(&ziel, &[]).expect("der Export gelingt");

    assert_ne!(fs::read_to_string(&ziel).expect("lesbar"), "alt");
    assert_eq!(
        fs::read_dir(&ordner.0).expect("lesbar").count(),
        1,
        "nur die Export-Datei"
    );
}

#[cfg(unix)]
#[test]
fn scheitert_das_schreiben_bleibt_die_alte_datei_unberuehrt() {
    use std::os::unix::fs::PermissionsExt;
    let ordner = Ordner::neu("scheitern");
    let ziel = ordner.0.join("anfrage.json");
    fs::write(&ziel, "alt").expect("eine alte Datei");
    // Ein schreibgeschützter Ordner: die Nachbar-Datei lässt sich nicht anlegen.
    fs::set_permissions(&ordner.0, fs::Permissions::from_mode(0o500)).expect("schreibgeschützt");

    let fehler = write(&ziel, &[]);
    fs::set_permissions(&ordner.0, fs::Permissions::from_mode(0o700)).expect("wieder beschreibbar");

    let fehler = fehler.expect_err("das Schreiben scheitert");
    assert_eq!(fs::read_to_string(&ziel).expect("lesbar"), "alt");
    // Redigiert: kein Pfad in der Meldung.
    assert!(!fehler.to_string().contains("anfrage"));
}
