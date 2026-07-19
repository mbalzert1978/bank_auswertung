use fenstern_domain::{AnalysisWindow, AnalysisWindowRepository, WindowError, WindowId};

use crate::abstractions::{AnalysisWindowStoreSeam, StoreUnavailable};
use crate::models::AnalysisWindowRow;

/// Die **Port-Bridge** für die Ablage: erfüllt `fenstern_domain::AnalysisWindowRepository` und setzt
/// das Domänen-Aggregat auf die [`AnalysisWindowRow`] um (ACL, „innen → außen"), die sie in den
/// öffentlichen [`AnalysisWindowStoreSeam`] staged (den io — sqlx — erfüllt). Die vom Seam **neu
/// vergebene** ID hüllt sie „außen → innen" in die [`WindowId`] und reicht sie an den Handler
/// zurück, der genau sie als Zeiger publiziert; ein `Err` des Seams ist **terminal** →
/// `WindowError::StorageUnavailable` mit durchgereichter io-Diagnose.
pub(crate) struct RepositoryBridge<'a> {
    pub(crate) seam: &'a dyn AnalysisWindowStoreSeam,
}

impl AnalysisWindowRepository for RepositoryBridge<'_> {
    fn save(&self, window: &AnalysisWindow) -> Result<WindowId, WindowError> {
        self.seam
            .save(&AnalysisWindowRow::from(window))
            .map(WindowId::new)
            .map_err(
                |StoreUnavailable { message }| WindowError::StorageUnavailable { reason: message },
            )
    }
}
