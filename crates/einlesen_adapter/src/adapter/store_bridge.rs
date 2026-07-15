use einlesen_domain::{IngestionId, LineAggregate, SinkError, Store};

use crate::abstractions::{StoreSeam, StoreUnavailable};
use crate::models::Line;

/// Port-Bridge: mappt den öffentlichen [`StoreSeam`] (den io erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Store`, den der Handler verlangt — analog
/// [`CalendarBridge`](crate::adapter::CalendarBridge). Das Aggregat (Domänen-VO) wird
/// **hier** auf das [`Line`]-DTO abgeflacht (ACL, „innen → außen"), bevor es an den Seam geht;
/// der Handler kennt nur den Domänen-Port. Die vom Seam beim Ablegen **vergebene ID** hüllt sie
/// „außen → innen" in die [`IngestionId`] und reicht sie an den Handler zurück, der sie als Zeiger
/// weiterpubliziert. Ein `Err` des Seams ist **terminal** → [`SinkError::StoreFailed`] mit
/// durchgereichter io-Diagnose.
pub(crate) struct StoreBridge<'a> {
    pub(crate) seam: &'a dyn StoreSeam,
}

impl Store for StoreBridge<'_> {
    fn store(&self, aggregate: &LineAggregate) -> Result<IngestionId, SinkError> {
        self.seam
            .store(&Line::from(aggregate))
            .map(IngestionId::new)
            .map_err(|StoreUnavailable { message }| SinkError::StoreFailed { reason: message })
    }
}
