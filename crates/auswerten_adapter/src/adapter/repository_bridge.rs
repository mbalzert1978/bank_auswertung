use auswerten_domain::{EvaluationError, EvaluationRepository, WindowEvaluation};

use crate::abstractions::{EvaluationStoreSeam, StoreUnavailable};
use crate::models::WindowEvaluationRow;

/// Die **Port-Bridge** für die Ablage: erfüllt `auswerten_domain::EvaluationRepository` und setzt das
/// Domänen-Aggregat auf die [`WindowEvaluationRow`] um (ACL, „innen → außen"), die sie in den
/// öffentlichen [`EvaluationStoreSeam`] staged (den io — sqlx — erfüllt). Ein `Err` des Seams ist
/// **terminal** → `EvaluationError::StorageUnavailable` mit durchgereichter io-Diagnose.
pub(crate) struct RepositoryBridge<'a> {
    pub(crate) seam: &'a dyn EvaluationStoreSeam,
}

impl EvaluationRepository for RepositoryBridge<'_> {
    fn save(&self, evaluation: &WindowEvaluation) -> Result<(), EvaluationError> {
        self.seam
            .save(&WindowEvaluationRow::from(evaluation))
            .map_err(
                |StoreUnavailable { message }| EvaluationError::StorageUnavailable {
                    reason: message,
                },
            )
    }
}
