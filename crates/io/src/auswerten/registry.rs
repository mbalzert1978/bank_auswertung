//! Das reale **Kategorie-Register**
//! ([Issue 0009](../../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md),
//! D5): was die Domäne über eine Buchungs*art* **gelernt** hat — Fingerabdruck → Kategorie —, in
//! **einer** SQLite-Tabelle, die über jeden Neustart trägt.
//!
//! **Geschlüsselt auf den Fingerabdruck, nie auf Text.** Die Tabelle hat genau zwei Fachspalten: den
//! Fingerabdruck (Primärschlüssel) und das Etikett. Einen Text, aus dem das Etikett geraten wurde,
//! gibt es hier nicht — der Rückweg vom Etikett auf einen Freitext existiert damit auch auf der Platte
//! nicht (ADR-0002). Der Fingerabdruck selbst ist ein Derivat: eine öffentliche Gläubiger-ID oder ein
//! seed-basiertes Pseudonym, nie ein Rohwert und **nie der Seed** (ADR-0010).
//!
//! **Zwei Wege hinein und heraus, beide hier:**
//!
//! - **Lesen** im Lauf über den [`CategoryRegistrySeam`]: welche dieser Fingerabdrücke kenne ich
//!   schon? Die Domäne fragt das Orakel danach nur noch für den Rest (ADR-0004). Gelesen wird außerhalb
//!   der committenden Transaktion, wie jede Lese-Quelle (eine reine Leihgabe der Verbindung).
//! - **Lernen** über [`SqliteCategoryRegistry::learn`]: der Schreibweg für den Import der
//!   Chat-Antwort ([Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md))
//!   und für jede Korrektur von Hand. Eine Korrektur **ersetzt** das alte Etikett desselben
//!   Fingerabdrucks — das Register hält je Art genau eine, die zuletzt gelernte Kategorie.
//!
//! **Die Kategorie ist freier, validierter Text, kein festes Vokabular.** So hält es schon die Domäne
//! (`auswerten_domain::Category` über `NonEmptyString`): welche Schubladen es gibt, ist eine Aussage
//! des Orakels bzw. des Menschen, nicht der Analyse. Validiert wird darum genau das, was die Domäne
//! verlangt — getrimmt und nicht leer —, und ein `CHECK` hält dasselbe auf DB-Ebene.

use auswerten_adapter::{CategoryAnswerRow, CategoryRegistrySeam, RegistryUnavailable};
use rusqlite::{Connection, OptionalExtension};

use crate::sqlite::{SqliteOutbox, TableSpec};

/// Die Tabelle des Registers — beim Schema-Besitzer ([`schema`](crate::schema)) aufgeführt, der sie
/// anlegt (und eine Version-2-Datei um sie ergänzt).
///
/// **Ein natürlicher Schlüssel statt einer geminteten ID:** die Identität eines Eintrags **ist** sein
/// Fingerabdruck — eine zweite, generierte ID ließe zwei Einträge derselben Art zu, und genau das
/// wäre ein widersprüchliches Register.
pub(crate) const LEARNED_CATEGORIES_TABLE: TableSpec = TableSpec {
    name: "learned_categories",
    ddl: LEARNED_CATEGORIES_DDL,
    insert: LEARN,
};

const LEARNED_CATEGORIES_DDL: &str = "\
CREATE TABLE IF NOT EXISTS learned_categories (
    fingerprint TEXT    PRIMARY KEY CHECK (trim(fingerprint) <> ''),
    category    TEXT    NOT NULL    CHECK (trim(category) <> ''),
    created_at  INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    updated_at  INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))
);";

/// Lernt ein Etikett — und **korrigiert** ein vorhandenes: derselbe Fingerabdruck bekommt das neue
/// Etikett, `updated_at` rückt nach. Dasselbe Etikett erneut zu lernen ändert nichts (auch nicht den
/// Zeitstempel): ein wiederholter Import ist damit harmlos.
const LEARN: &str = "\
INSERT INTO learned_categories (fingerprint, category) VALUES (?1, ?2)
ON CONFLICT (fingerprint) DO UPDATE
SET category = excluded.category,
    updated_at = CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE category <> excluded.category;";

const SELECT_CATEGORY: &str = "SELECT category FROM learned_categories WHERE fingerprint = ?1;";

/// Das reale Kategorie-Register auf dem SQLite-Rückgrat. `Clone` teilt dasselbe Rückgrat (`Rc` im
/// Innern von [`SqliteOutbox`]). Legt **nichts** an — die Tabelle gehört dem Schema-Besitzer.
#[derive(Clone)]
pub struct SqliteCategoryRegistry {
    outbox: SqliteOutbox,
}

impl SqliteCategoryRegistry {
    /// Bindet das Register an das gegebene Rückgrat.
    #[must_use]
    pub fn new(outbox: SqliteOutbox) -> Self {
        Self { outbox }
    }

    /// Lernt die gegebenen Etiketten — **alle oder keines**, in einer Transaktion. Ein schon bekannter
    /// Fingerabdruck wird **korrigiert** (das neue Etikett ersetzt das alte); das übersteht jeden
    /// Neustart, und der nächste Lauf fragt das Orakel nach dieser Art nicht mehr.
    ///
    /// Die Eingabe ist die **Antwort-Zeile des Orakels** ([`CategoryAnswerRow`]) — dieselbe Form, die
    /// der Import der Chat-Antwort (Issue 0011) liefert. Fingerabdruck und Etikett werden getrimmt
    /// abgelegt.
    ///
    /// # Errors
    ///
    /// [`LearnError::Rejected`], wenn ein Eintrag einen leeren Fingerabdruck oder ein leeres Etikett
    /// trägt oder derselbe Fingerabdruck im Stapel zwei verschiedene Etiketten bekommt — dann wird
    /// **nichts** gelernt. [`LearnError::Unavailable`], wenn das Register nicht beschreibbar war.
    pub fn learn(&self, answers: &[CategoryAnswerRow]) -> Result<(), LearnError> {
        let entries = validated(answers)?;
        self.outbox
            .with_connection(|conn| {
                let transaction = conn.unchecked_transaction()?;
                {
                    let mut statement = transaction.prepare(LEARN)?;
                    entries.iter().try_for_each(|(fingerprint, category)| {
                        statement.execute((fingerprint, category)).map(drop)
                    })?;
                }
                transaction.commit()
            })
            .map_err(|error| LearnError::Unavailable(error.to_string()))
    }

    /// Die gelernten Etiketten der gegebenen Fingerabdrücke, in deren Reihenfolge; ein unbekannter
    /// fehlt schlicht. Der Lese-Weg für alles, was das Etikett **zeigen** will (der Bericht, Issue
    /// 0012) — die Domäne selbst fragt nur, **ob** eine Art bekannt ist.
    ///
    /// # Errors
    ///
    /// [`RegistryUnavailable`], wenn das Register nicht lesbar war (io-Diagnose, nie ein Schlüssel).
    pub fn categories_of(
        &self,
        fingerprints: &[String],
    ) -> Result<Vec<CategoryAnswerRow>, RegistryUnavailable> {
        self.outbox
            .with_connection(|conn| lookup(conn, fingerprints))
            .map_err(|error| RegistryUnavailable {
                message: error.to_string(),
            })
    }
}

impl CategoryRegistrySeam for SqliteCategoryRegistry {
    /// Die Teilmenge der Kandidaten, die das Register führt. Wieder zu Domänen-Schlüsseln werden sie
    /// erst in der `RegistryBridge` des Adapters — durch Abgleich mit den gestellten Kandidaten, nie
    /// durch Neubildung aus einer Zeichenkette (ACL).
    fn known_among(&self, candidates: &[String]) -> Result<Vec<String>, RegistryUnavailable> {
        self.categories_of(candidates)
            .map(|known| known.into_iter().map(|answer| answer.fingerprint).collect())
    }
}

/// Je Kandidat **eine** indizierte Abfrage über den Primärschlüssel, mit **einem** vorbereiteten
/// Statement — die Zahl der Buchungsarten eines Fensters ist klein, und ein dynamisches `IN (…)` wäre
/// mehr SQL-Bau als Gewinn.
fn lookup(conn: &Connection, fingerprints: &[String]) -> rusqlite::Result<Vec<CategoryAnswerRow>> {
    let mut statement = conn.prepare(SELECT_CATEGORY)?;
    fingerprints
        .iter()
        .filter_map(|fingerprint| {
            statement
                .query_row([fingerprint], |row| row.get::<_, String>(0))
                .optional()
                .transpose()
                .map(|category| {
                    category.map(|category| CategoryAnswerRow {
                        fingerprint: fingerprint.clone(),
                        category,
                    })
                })
        })
        .collect()
}

/// Prüft den ganzen Stapel **vor** dem ersten Schreiben: getrimmt, nicht leer, und je Fingerabdruck
/// höchstens ein Etikett. Ein Widerspruch im selben Stapel ist keine Korrektur, sondern eine unklare
/// Eingabe — welches Etikett „gemeint" war, rät das Register nicht.
fn validated(answers: &[CategoryAnswerRow]) -> Result<Vec<(String, String)>, LearnError> {
    answers.iter().enumerate().try_fold(
        Vec::<(String, String)>::new(),
        |mut entries, (index, answer)| {
            let position = index + 1;
            let fingerprint = non_empty(&answer.fingerprint).ok_or(LearnError::Rejected {
                position,
                cause: RejectCause::EmptyFingerprint,
            })?;
            let category = non_empty(&answer.category).ok_or(LearnError::Rejected {
                position,
                cause: RejectCause::EmptyCategory,
            })?;
            match entries.iter().find(|(known, _)| *known == fingerprint) {
                Some((_, earlier)) if *earlier != category => Err(LearnError::Rejected {
                    position,
                    cause: RejectCause::ContradictingCategory,
                }),
                Some(_) => Ok(entries),
                None => {
                    entries.push((fingerprint, category));
                    Ok(entries)
                }
            }
        },
    )
}

fn non_empty(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Warum das Register einen Stapel nicht gelernt hat. **Redigiert:** die Meldung nennt die Position
/// des Eintrags im Stapel und einen statischen Grund, **nie** einen Fingerabdruck oder ein Etikett.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LearnError {
    /// Ein Eintrag ist ungültig; gelernt wurde **nichts**. `position` zählt ab 1.
    Rejected { position: usize, cause: RejectCause },
    /// Das Register war nicht beschreibbar — die io-Diagnose (Zugriffs-Mechanik).
    Unavailable(String),
}

/// Der statische Grund einer Ablehnung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectCause {
    /// Der Fingerabdruck ist leer.
    EmptyFingerprint,
    /// Das Etikett ist leer — eine leere Kategorie ist keine Kategorie.
    EmptyCategory,
    /// Derselbe Fingerabdruck bekommt im Stapel ein zweites, anderes Etikett.
    ContradictingCategory,
}

impl std::fmt::Display for LearnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected { position, cause } => {
                let reason = match cause {
                    RejectCause::EmptyFingerprint => "leerer Fingerabdruck",
                    RejectCause::EmptyCategory => "leere Kategorie",
                    RejectCause::ContradictingCategory => {
                        "widersprüchliche Kategorie für denselben Fingerabdruck"
                    }
                };
                write!(
                    f,
                    "Kategorie-Register: Eintrag {position} abgewiesen ({reason}) — nichts gelernt"
                )
            }
            Self::Unavailable(diagnosis) => {
                write!(f, "Kategorie-Register nicht verfügbar: {diagnosis}")
            }
        }
    }
}

impl std::error::Error for LearnError {}

#[cfg(test)]
mod tests;
