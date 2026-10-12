//! Der **Import der Chat-Antwort**
//! ([Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)): die Datei, die
//! der Nutzer aus dem Chat zurückträgt, wird gelesen, geprüft und über
//! [`SqliteCategoryRegistry::learn`] ins Kategorie-Register übernommen — den **einzigen** Schreibweg.
//!
//! **Zugeordnet wird nur über den Fingerabdruck, exakt.** Kein Vergleich ohne Groß/klein, kein
//! Präfix, keine Ähnlichkeit, keine Suche über Text. Ein Eintrag zählt nur mit den Feldern
//! `fingerabdruck` und `kategorie`; jedes weitere Feld (`text`, `begruendung` …) wird **nie gelesen**,
//! nur gezählt. Eine Antwort ohne passenden Fingerabdruck wird verworfen — nie über ihren Text
//! zugeordnet.
//!
//! **Die Prüfung, Schritt für Schritt** (jede Meldung nennt nur Positionen und feste Gründe, nie einen
//! Wert aus der Datei):
//!
//! 1. Datei zu groß, kein JSON, oder oben keine `antworten`-Liste → die **ganze Datei** abgewiesen.
//! 2. Ein Eintrag ohne `fingerabdruck`/`kategorie` als Text, oder mit ungültiger [`Kategorie`] →
//!    **verworfen**.
//! 3. **Widerspruch** über alle formal gültigen Einträge, **vor** den Filtern: bekommt ein
//!    Fingerabdruck zwei verschiedene Kategorien, ist die ganze Datei abgewiesen — welche gemeint
//!    war, rät das Programm nicht.
//! 4. Ein Fingerabdruck, den keine Auswertung ausgegeben hat → verworfen.
//! 5. Ohne Korrektur-Schalter: ein Fingerabdruck, den das Register schon kennt → **übergangen**, sein
//!    Etikett bleibt. Eine bewusste Korrektur braucht den Schalter.
//! 6. Der Rest geht in **einem** Aufruf an `learn` — alles oder nichts; dessen Prüfungen bleiben das
//!    letzte Netz.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::io::Read;
use std::path::Path;

use auswerten_adapter::{CategoryAnswerRow, CategoryRegistrySeam};
use serde_json::{Map, Value};

use crate::auswerten::SqliteCategoryRegistry;

/// Die größte Antwort-Datei, die gelesen wird. Eine Antwort trägt je Buchungsart eine kurze Zeile —
/// ein MiB fasst Tausende davon; was größer ist, ist keine Antwort auf diesen Export.
pub const MAX_ANSWER_BYTES: u64 = 1 << 20;

/// Die längste zulässige Kategorie in Zeichen.
const MAX_CATEGORY_CHARS: usize = 60;

/// Ob der Import ein schon bekanntes Etikett ersetzen darf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Nur Unbekanntes lernen; ein bekanntes Etikett bleibt (der Normalfall nach einem Export).
    LearnUnknown,
    /// Auch bekannte Etiketten ersetzen — der bewusste Weg für eine Korrektur.
    Correct,
}

/// Eine **Kategorie aus einer Chat-Antwort** — das Value Object am Rand: getrimmt 1–60 Zeichen,
/// einzeilig, ohne Steuerzeichen (auch ohne Zeilen-/Absatztrenner und Bidi-Steuerzeichen, die eine
/// Anzeige verdrehen könnten). Mehr prüft es **nicht**: HTML-sicher macht das Etikett erst der
/// Bericht, der es escaped (Issue 0012).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Kategorie(String);

impl Kategorie {
    /// Parst ein Etikett; `None`, wenn es die Regeln oben verletzt.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        let length = trimmed.chars().count();
        ((1..=MAX_CATEGORY_CHARS).contains(&length) && !trimmed.chars().any(forbidden))
            .then(|| Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Steuerzeichen, Zeilen-/Absatztrenner und Bidi-Steuerzeichen.
fn forbidden(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{2028}' | '\u{2029}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
        )
}

/// Warum ein einzelner Eintrag verworfen wurde — ein fester Grund, nie ein Wert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscardCause {
    /// Der Eintrag ist kein Objekt.
    NotAnObject,
    /// `fingerabdruck` fehlt, ist leer oder kein Text.
    MissingFingerprint,
    /// `kategorie` fehlt oder ist kein Text.
    MissingCategory,
    /// `kategorie` verletzt die Regeln der [`Kategorie`].
    InvalidCategory,
    /// Den Fingerabdruck hat keine Auswertung ausgegeben.
    NotIssued,
}

impl std::fmt::Display for DiscardCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotAnObject => "kein Objekt",
            Self::MissingFingerprint => "Fingerabdruck fehlt",
            Self::MissingCategory => "Kategorie fehlt",
            Self::InvalidCategory => "Kategorie ungültig",
            Self::NotIssued => "Fingerabdruck nie ausgegeben",
        })
    }
}

/// Was der Import getan hat. Positionen zählen ab 1 in der `antworten`-Liste der Datei.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// Wie viele Fingerabdrücke gelernt (oder mit dem Korrektur-Schalter ersetzt) wurden.
    pub learned: usize,
    /// Die Positionen der Einträge, deren Fingerabdruck schon ein Etikett trug — übergangen.
    pub kept_known: Vec<usize>,
    /// Die verworfenen Einträge mit ihrem Grund.
    pub discarded: Vec<(usize, DiscardCause)>,
    /// Wie viele Felder außer `fingerabdruck`/`kategorie` übergangen wurden (nie gelesen).
    pub ignored_fields: usize,
}

/// Warum die **ganze** Datei abgewiesen wurde — nichts wurde gelernt. **Redigiert:** feste Ursachen,
/// Positionen, Zeile/Spalte; nie ein Wert aus der Datei, nie der Pfad, nie der Fehlertext von
/// `serde_json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// Die Datei ließ sich nicht lesen (Zugriffs-Mechanik).
    Unreadable(String),
    /// Die Datei ist größer als [`MAX_ANSWER_BYTES`].
    TooLarge,
    /// Kein gültiges JSON.
    Malformed { line: usize, column: usize },
    /// Oben steht kein Objekt mit einer `antworten`-Liste.
    NoAnswerList,
    /// Derselbe Fingerabdruck bekommt zwei verschiedene Kategorien.
    Contradiction { first: usize, second: usize },
    /// Das Register war nicht lesbar oder hat den Stapel abgewiesen.
    Registry(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(diagnosis) => write!(f, "Antwort-Datei nicht lesbar: {diagnosis}"),
            Self::TooLarge => write!(
                f,
                "Antwort-Datei größer als {MAX_ANSWER_BYTES} Bytes — das ist keine Antwort auf \
                 einen Export"
            ),
            Self::Malformed { line, column } => write!(
                f,
                "Antwort-Datei ist kein gültiges JSON (Zeile {line}, Spalte {column})"
            ),
            Self::NoAnswerList => {
                f.write_str("Antwort-Datei trägt oben kein Objekt mit einer `antworten`-Liste")
            }
            Self::Contradiction { first, second } => write!(
                f,
                "Einträge {first} und {second} geben demselben Fingerabdruck verschiedene \
                 Kategorien — nichts gelernt"
            ),
            Self::Registry(diagnosis) => write!(f, "{diagnosis}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// Ein formal gültiger Eintrag: Position, Fingerabdruck (getrimmt), Kategorie.
struct Candidate {
    position: usize,
    fingerprint: String,
    category: Kategorie,
}

/// Liest die Antwort-Datei und übernimmt sie ins Register (siehe Modul-Doku).
///
/// # Errors
///
/// [`ImportError`], wenn die ganze Datei abgewiesen wurde — dann ist **nichts** gelernt.
pub fn import(
    registry: &SqliteCategoryRegistry,
    answers: &Path,
    mode: ImportMode,
) -> Result<ImportReport, ImportError> {
    let document = parse(&read_bounded(answers)?)?;
    let entries = answer_list(&document)?;
    let mut report = ImportReport {
        ignored_fields: document
            .as_object()
            .map_or(0, |top| top.len().saturating_sub(1)),
        ..ImportReport::default()
    };
    let candidates = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| match candidate(index + 1, entry) {
            Ok((candidate, ignored)) => {
                report.ignored_fields += ignored;
                Some(candidate)
            }
            Err(discard) => {
                report.discarded.push(discard);
                None
            }
        })
        .collect::<Vec<_>>();
    let unique = without_contradiction(candidates)?;
    let fingerprints = unique
        .iter()
        .map(|candidate| candidate.fingerprint.clone())
        .collect::<Vec<_>>();
    let issued = registry.issued_among(&fingerprints).map_err(unavailable)?;
    let known = match mode {
        ImportMode::LearnUnknown => registry.known_among(&fingerprints).map_err(unavailable)?,
        ImportMode::Correct => Vec::new(),
    };
    let learnable = unique
        .into_iter()
        .filter(|candidate| {
            if !issued.contains(&candidate.fingerprint) {
                report
                    .discarded
                    .push((candidate.position, DiscardCause::NotIssued));
                false
            } else if known.contains(&candidate.fingerprint) {
                report.kept_known.push(candidate.position);
                false
            } else {
                true
            }
        })
        .map(|candidate| CategoryAnswerRow {
            fingerprint: candidate.fingerprint,
            category: candidate.category.0,
        })
        .collect::<Vec<_>>();
    registry
        .learn(&learnable)
        .map_err(|error| ImportError::Registry(error.to_string()))?;
    report.learned = learnable.len();
    report.discarded.sort_unstable();
    report.kept_known.sort_unstable();
    Ok(report)
}

/// Liest höchstens [`MAX_ANSWER_BYTES`] + 1 Bytes — eine größere Datei wird gar nicht erst ganz
/// geladen.
fn read_bounded(path: &Path) -> Result<String, ImportError> {
    let unreadable = |error: std::io::Error| ImportError::Unreadable(error.kind().to_string());
    let mut content = String::new();
    std::fs::File::open(path)
        .map_err(unreadable)?
        .take(MAX_ANSWER_BYTES + 1)
        .read_to_string(&mut content)
        .map_err(unreadable)?;
    match u64::try_from(content.len()).map_or(true, |length| length > MAX_ANSWER_BYTES) {
        true => Err(ImportError::TooLarge),
        false => Ok(content),
    }
}

/// Parst das JSON — der Fehler trägt nur Zeile und Spalte, nie den Text von `serde_json` (er kann
/// Inhalt zitieren).
fn parse(content: &str) -> Result<Value, ImportError> {
    serde_json::from_str(content).map_err(|error| ImportError::Malformed {
        line: error.line(),
        column: error.column(),
    })
}

fn answer_list(document: &Value) -> Result<&Vec<Value>, ImportError> {
    document
        .get("antworten")
        .and_then(Value::as_array)
        .ok_or(ImportError::NoAnswerList)
}

/// Prüft einen Eintrag: nur `fingerabdruck` und `kategorie` werden gelesen; die Zahl der übrigen Felder
/// kommt mit zurück.
fn candidate(position: usize, entry: &Value) -> Result<(Candidate, usize), (usize, DiscardCause)> {
    let object: &Map<String, Value> = entry
        .as_object()
        .ok_or((position, DiscardCause::NotAnObject))?;
    let fingerprint = object
        .get("fingerabdruck")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|fingerprint| !fingerprint.is_empty())
        .ok_or((position, DiscardCause::MissingFingerprint))?;
    let category = object
        .get("kategorie")
        .and_then(Value::as_str)
        .ok_or((position, DiscardCause::MissingCategory))?;
    let category = Kategorie::parse(category).ok_or((position, DiscardCause::InvalidCategory))?;
    let ignored = object
        .keys()
        .filter(|key| !matches!(key.as_str(), "fingerabdruck" | "kategorie"))
        .count();
    Ok((
        Candidate {
            position,
            fingerprint: fingerprint.to_string(),
            category,
        },
        ignored,
    ))
}

/// Faltet gleiche Einträge zusammen und weist einen **Widerspruch** ab — über alle formal gültigen
/// Einträge, bevor irgendetwas gefiltert wird, mit den Positionen aus der Datei.
fn without_contradiction(candidates: Vec<Candidate>) -> Result<Vec<Candidate>, ImportError> {
    candidates
        .into_iter()
        .try_fold(
            BTreeMap::<String, Candidate>::new(),
            |mut unique, candidate| match unique.entry(candidate.fingerprint.clone()) {
                Entry::Vacant(slot) => {
                    slot.insert(candidate);
                    Ok(unique)
                }
                Entry::Occupied(first) if first.get().category == candidate.category => Ok(unique),
                Entry::Occupied(first) => Err(ImportError::Contradiction {
                    first: first.get().position,
                    second: candidate.position,
                }),
            },
        )
        .map(|unique| unique.into_values().collect())
}

fn unavailable(error: auswerten_adapter::RegistryUnavailable) -> ImportError {
    ImportError::Registry(format!(
        "Kategorie-Register nicht verfügbar: {}",
        error.message
    ))
}

#[cfg(test)]
mod tests;
