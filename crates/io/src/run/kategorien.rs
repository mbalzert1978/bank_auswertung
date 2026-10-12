//! Die beiden Rand-Schritte des Chat-Kategorisierers
//! ([Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)), so wie das
//! Programm sie fährt: der **Export** am Ende eines gelungenen Laufs und der **Import** als eigener
//! Befehl. Die Mechanik selbst liegt in [`chat_export`] und [`chat_import`]; hier steht nur, was der
//! Lauf darum herum tut — das Register ein zweites Mal fragen, die Datenbank öffnen, das redigierte
//! Protokoll schreiben.

use std::io::Write;
use std::path::{Path, PathBuf};

use auswerten_adapter::{CategoryRegistrySeam, CategoryRequestRow};

use super::{RunError, write_line};
use crate::auswerten::SqliteCategoryRegistry;
use crate::auswerten::chat_export;
use crate::auswerten::chat_import::{self, ImportMode, ImportReport};
use crate::config::AppConfig;
use crate::schema;

/// Schreibt den Export der gesammelten Anfragen nach `target` — vorher fragt es das Register ein
/// **zweites** Mal und lässt Bekanntes weg (doppelte Sicherung; die Domäne hat schon gefiltert).
pub(super) fn export(
    target: &Path,
    registry: &SqliteCategoryRegistry,
    collected: &[CategoryRequestRow],
    protocol: &mut dyn Write,
) -> Result<(), RunError> {
    let known = registry
        .known_among(
            &collected
                .iter()
                .map(|request| request.fingerprint().to_string())
                .collect::<Vec<_>>(),
        )
        .map_err(|error| RunError::Backbone(error.message))?;
    let unknown = collected
        .iter()
        .filter(|request| {
            !known
                .iter()
                .any(|fingerprint| fingerprint == request.fingerprint())
        })
        .cloned()
        .collect::<Vec<_>>();
    chat_export::write(target, &unknown).map_err(|error| RunError::Export(error.to_string()))?;
    write_line(
        protocol,
        &format!(
            "Kategorien-Export: {} unbekannte Buchungsart(en) — die Datei vor dem Einfügen in den \
             Chat ansehen",
            unknown.len()
        ),
    )
}

/// Wo die Eingaben des Imports liegen — die Datenbank und die Antwort-Datei aus dem Chat.
#[derive(Debug, Clone)]
pub struct ImportPaths {
    /// Die SQLite-Datei der Läufe; ihr Kategorie-Register lernt.
    pub database: PathBuf,
    /// Die Antwort-Datei aus dem Chat.
    pub answers: PathBuf,
}

/// Fährt den **Import** einer Chat-Antwort und schreibt das redigierte Protokoll: Zählwerte,
/// Positionen, feste Gründe — nie ein Fingerabdruck, nie ein Etikett.
///
/// # Errors
///
/// - [`RunError::SchemaVersion`]/[`RunError::Backbone`], wenn die Datenbank nicht passt;
/// - [`RunError::Import`], wenn die ganze Datei abgewiesen wurde (nichts gelernt);
/// - [`RunError::NothingLearned`], wenn **kein** Eintrag gelernt, aber mindestens einer verworfen
///   wurde — dann stimmt mit der Antwort etwas nicht, und ein Exit 0 verdeckte das.
pub fn import_categories(
    paths: &ImportPaths,
    mode: ImportMode,
    protocol: &mut dyn Write,
) -> Result<ImportReport, RunError> {
    let outbox = schema::open(&AppConfig::new(&paths.database))?;
    let registry = SqliteCategoryRegistry::new(outbox);
    let report = chat_import::import(&registry, &paths.answers, mode).map_err(RunError::Import)?;
    write_line(protocol, &summary(&report, mode))?;
    match report.learned == 0 && !report.discarded.is_empty() {
        true => Err(RunError::NothingLearned {
            discarded: report.discarded.len(),
        }),
        false => Ok(report),
    }
}

/// Die eine Protokoll-Zeile des Imports.
fn summary(report: &ImportReport, mode: ImportMode) -> String {
    let positions = |positions: &[usize]| {
        positions
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut line = format!("Kategorien-Import: {} gelernt", report.learned);
    if !report.kept_known.is_empty() {
        line.push_str(&format!(
            ", {} schon bekannt und nicht ersetzt (Einträge {})",
            report.kept_known.len(),
            positions(&report.kept_known)
        ));
        if mode == ImportMode::LearnUnknown {
            line.push_str(" — ersetzen nur mit --korrektur");
        }
    }
    if !report.discarded.is_empty() {
        let reasons = report
            .discarded
            .iter()
            .map(|(position, cause)| format!("Eintrag {position}: {cause}"))
            .collect::<Vec<_>>()
            .join("; ");
        line.push_str(&format!(
            ", {} verworfen ({reasons})",
            report.discarded.len()
        ));
    }
    if report.ignored_fields > 0 {
        line.push_str(&format!(
            ", {} Zusatzfeld(er) nicht gelesen",
            report.ignored_fields
        ));
    }
    line
}
