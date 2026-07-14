use einlesen_domain::{IngestionId, LineAggregate, Store};

use crate::abstractions::StoreSeam;
use crate::dto::Line;

/// Port-Bridge: mappt den öffentlichen [`StoreSeam`] (den io erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Store`, den der Handler verlangt — analog
/// [`CalendarBridge`](crate::bridges::CalendarBridge). Das Aggregat (Domänen-VO) wird
/// **hier** auf das [`Line`]-DTO abgeflacht (ACL, „innen → außen"), bevor es an den Seam geht;
/// der Handler kennt nur den Domänen-Port. Die vom Seam beim Ablegen **vergebene ID** hüllt sie
/// „außen → innen" in die [`IngestionId`] und reicht sie an den Handler zurück, der sie als Zeiger
/// weiterpubliziert.
pub(crate) struct StoreBridge<'a> {
    pub(crate) seam: &'a dyn StoreSeam,
}

impl Store for StoreBridge<'_> {
    fn store(&self, aggregate: &LineAggregate) -> IngestionId {
        IngestionId::new(self.seam.store(&Line::from(aggregate)))
    }
}
