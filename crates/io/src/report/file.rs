//! Das Schreiben der Bericht-Datei: erst in eine **Nachbar-Datei**, dann per Umbenennen auf das Ziel.
//! Auf demselben Dateisystem ist das Umbenennen atomar — am Ziel liegt entweder der alte Bericht oder
//! der neue, nie ein halber.

use std::ffi::OsString;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

/// Ersetzt `target` durch `content`. Scheitert ein Schritt, bleibt eine ältere Datei am Ziel
/// unberührt und die Nachbar-Datei wird (best effort) weggeräumt.
pub(super) fn replace(target: &Path, content: &str) -> std::io::Result<()> {
    let neighbour = neighbour(target)?;
    let written =
        std::fs::write(&neighbour, content).and_then(|()| std::fs::rename(&neighbour, target));
    if written.is_err() {
        // Best effort: eine schon fehlende Nachbar-Datei ist kein zweiter Fehler; zurück kommt der
        // eigentliche.
        match std::fs::remove_file(&neighbour) {
            Ok(()) | Err(_) => {}
        }
    }
    written
}

/// Die Nachbar-Datei: derselbe Ordner, der Dateiname mit angehängtem `.tmp`.
pub(crate) fn neighbour(target: &Path) -> std::io::Result<PathBuf> {
    let name = target.file_name().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "der Bericht-Pfad nennt keinen Dateinamen",
        )
    })?;
    let mut temporary = OsString::from(name);
    temporary.push(".tmp");
    Ok(target.with_file_name(temporary))
}
