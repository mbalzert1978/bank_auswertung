use einlesen_domain::{LineAggregate, Store};

use crate::abstractions::StoreSeam;
use crate::dto::Line;

/// Port-Bridge: mappt den öffentlichen [`StoreSeam`] (den io erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Store`, den der Handler verlangt — analog
/// [`CalendarBridge`](crate::bridges::CalendarBridge). Das Aggregat (Domänen-VO) wird
/// **hier** auf das [`Line`]-DTO abgeflacht (ACL, „innen → außen"), bevor es an den Seam geht;
/// der Handler kennt nur den Domänen-Port.
pub(crate) struct StoreBridge<'a, S: StoreSeam> {
    pub(crate) seam: &'a S,
}

impl<S: StoreSeam> Store for StoreBridge<'_, S> {
    fn store(&self, aggregate: &LineAggregate) {
        self.seam.store(&Line::from_aggregate(aggregate));
    }
}
