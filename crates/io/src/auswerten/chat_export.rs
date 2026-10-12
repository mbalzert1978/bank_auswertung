//! Der **Export-Schreiber** des Chat-Kategorisierers
//! ([Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)) — die eine
//! Stelle, an der Daten dieses Programms für **außen** bestimmt sind.
//!
//! **Bewusst schmal.** Er nimmt nur das Ziel und die Anfrage-Zeilen, sonst nichts: keine Config,
//! keine Datenbank, keinen Seed. Was er nicht bekommt, kann er nicht hinausschreiben. Und eine
//! [`CategoryRequestRow`] kann nur die Domäne bauen (private Felder, einziger Konstruktor
//! `From<&CategoryRequest>`) — welcher Text hinausgeht, hat sie entschieden.
//!
//! **Kein `Serialize`.** Das JSON entsteht von Hand als `serde_json::Value` aus genau den zwei
//! benannten Feldern der Zeile. Bekommt die Zeile später ein drittes Feld, landet es **nicht** von
//! selbst in der Datei; und die Zeile selbst trägt den Guard
//! `assert_not_impl_any!(CategoryRequestRow: Serialize, …)`.
//!
//! **Nie eine halbe Datei, nie eine fremde überschrieben.** Geschrieben wird in eine **neue**
//! Nachbar-Datei mit Rechten `0600` (nur der Nutzer liest sie), dann umbenannt. Scheitert etwas, wird
//! die Nachbar-Datei gelöscht; eine vorhandene Export-Datei bleibt bis zur Umbenennung unberührt.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use auswerten_adapter::CategoryRequestRow;
use serde_json::{Map, Value, json};

/// Die Kennung des Formats — mit Versionsziffer, damit eine spätere Änderung sichtbar ist.
const FORMAT: &str = "bank-auswertung/kategorie-anfrage/1";

/// Die Anleitung für den Chat — ein fester Text, der keine Daten enthält.
const ANLEITUNG: &str = "Ordne jeder Anfrage genau eine kurze Kategorie zu (z. B. Lebensmittel, \
Wohnen, Mobilität). Antworte nur mit JSON in der Form {\"antworten\": [{\"fingerabdruck\": \"…\", \
\"kategorie\": \"…\"}]}. Übernimm jeden Fingerabdruck Zeichen für Zeichen. Lass eine Anfrage weg, \
wenn du sie nicht einordnen kannst.";

/// Warum der Export nicht geschrieben wurde. **Redigiert:** nur die Zugriffs-Mechanik — nie ein Pfad,
/// nie eine Anfrage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportError(String);

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Kategorien-Export nicht geschrieben: {}", self.0)
    }
}

impl std::error::Error for ExportError {}

/// Schreibt die Anfragen als Export-Datei nach `target` (siehe Modul-Doku).
///
/// # Errors
///
/// [`ExportError`], wenn die Nachbar-Datei nicht angelegt, beschrieben oder umbenannt werden konnte —
/// dann bleibt eine vorhandene Datei unter `target` unverändert.
pub fn write(target: &Path, requests: &[CategoryRequestRow]) -> Result<(), ExportError> {
    let content = render(requests);
    let staging = staging_path(target).ok_or_else(|| ExportError("ungültiges Ziel".into()))?;
    let written = create_private(&staging)
        .and_then(|mut file| {
            file.write_all(content.as_bytes())?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&staging, target));
    written.map_err(|error| {
        // Aufräumen ist ein Versuch: fehlt die Nachbar-Datei schon, ist nichts zu tun.
        let _cleanup: std::io::Result<()> = fs::remove_file(&staging);
        ExportError(error.kind().to_string())
    })
}

/// Das JSON der Export-Datei — je Anfrage genau `fingerabdruck` und `text`, von Hand benannt.
#[must_use]
pub fn render(requests: &[CategoryRequestRow]) -> String {
    let anfragen = requests
        .iter()
        .map(|request| {
            json!({
                "fingerabdruck": request.fingerprint(),
                "text": request.text(),
            })
        })
        .collect::<Vec<_>>();
    let document = Value::Object(Map::from_iter([
        ("format".to_string(), Value::from(FORMAT)),
        ("anleitung".to_string(), Value::from(ANLEITUNG)),
        ("anfragen".to_string(), Value::Array(anfragen)),
    ]));
    format!("{document:#}\n")
}

/// Die Nachbar-Datei: derselbe Ordner, ein eigener Name — eine Umbenennung im selben Ordner ist
/// atomar.
fn staging_path(target: &Path) -> Option<PathBuf> {
    let mut name = target.file_name()?.to_os_string();
    name.push(".neu");
    Some(target.with_file_name(name))
}

/// Legt die Nachbar-Datei **neu** an, mit `0600`. Eine von einem abgebrochenen Export liegengebliebene
/// wird vorher entfernt, nicht übernommen — sie trüge womöglich weitere Rechte.
fn create_private(path: &Path) -> std::io::Result<File> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
        Ok(()) | Err(_) => {}
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(test)]
mod tests;
