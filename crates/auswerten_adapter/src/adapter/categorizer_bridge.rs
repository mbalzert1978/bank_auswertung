use auswerten_domain::{
    Categorizer, Category, CategoryAnswer, CategoryRequest, EvaluationError, NonEmptyString,
};

use crate::abstractions::{CategorizerSeam, CategorizerUnavailable};
use crate::models::{CategoryAnswerRow, CategoryRequestRow};

/// Die **Port-Bridge** zum Orakel: erfüllt `auswerten_domain::Categorizer`, projiziert die Anfragen
/// auf das Egress-DTO [`CategoryRequestRow`] („innen → außen") und deutet die Antwort-Zeilen zurück
/// („außen → innen", ACL). Ein `Err` des Seams ist **terminal** →
/// `EvaluationError::CategorizerUnavailable` mit durchgereichter io-Diagnose.
pub(crate) struct CategorizerBridge<'a> {
    pub(crate) seam: &'a dyn CategorizerSeam,
}

impl Categorizer for CategorizerBridge<'_> {
    fn ask(&self, requests: &[CategoryRequest]) -> Result<Vec<CategoryAnswer>, EvaluationError> {
        let answers = self
            .seam
            .ask(
                &requests
                    .iter()
                    .map(CategoryRequestRow::from)
                    .collect::<Vec<_>>(),
            )
            .map_err(|CategorizerUnavailable { message }| {
                EvaluationError::CategorizerUnavailable { reason: message }
            })?;
        Ok(answers
            .iter()
            .filter_map(|answer| to_answer(answer, requests))
            .collect())
    }
}

/// Deutet **eine** Antwort-Zeile — indem sie gegen die **selbst gestellten** Anfragen gematcht wird.
///
/// Das ist die eigentliche Aussage dieser Funktion, nicht das Mapping: der Fingerabdruck der Antwort
/// wird **nicht** aus der Zeichenkette neu gebildet, sondern unter den Schlüsseln gesucht, nach denen
/// gefragt wurde. Zwei Dinge folgen daraus, und beide sind gewollt:
///
/// - **Das Orakel kann keinen Schlüssel erfinden.** Ein Etikett zu einem Fingerabdruck, nach dem nie
///   gefragt wurde, ist keine Antwort, sondern eine Behauptung — sie fällt hier lautlos weg. Der
///   Fingerabdruck bleibt damit Domänenwissen (ADR-0004) statt einer Absprache über Strings.
/// - **Eine leere Kategorie ist keine Kategorie.** Sie fällt weg wie eine ausgebliebene Antwort —
///   und nicht etwa als leeres Etikett in die Ablage, das später wie eine gelernte Deutung aussähe.
///
/// Beides ist ein **Auslassen, kein Fehlschlag**: das Orakel darf schweigen (siehe
/// `auswerten_domain::Categorizer`), und Unsinn von ihm den Lauf beenden zu lassen hieße, die
/// Auswertung von der Auskunftsfreude eines LLM abhängig zu machen.
fn to_answer(answer: &CategoryAnswerRow, requests: &[CategoryRequest]) -> Option<CategoryAnswer> {
    let fingerprint = requests
        .iter()
        .map(CategoryRequest::fingerprint)
        .find(|fingerprint| fingerprint.as_str() == answer.fingerprint)?;
    Some(CategoryAnswer::new(
        fingerprint.clone(),
        Category::from(NonEmptyString::new(&answer.category)?),
    ))
}
