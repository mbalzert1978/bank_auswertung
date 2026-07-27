use obfuskierung_domain::{
    BookingId, ObfuscatedBooking as DomainBooking, ObfuscatedRepository, SourceError,
};

use crate::abstractions::{ObfuscatedStoreSeam, StoreUnavailable};
use crate::models::ObfuscatedBooking;

/// Die **Port-Bridge** für die Ablage: erfüllt `obfuskierung_domain::ObfuscatedRepository` und setzt
/// das obfuskierte Domänen-Aggregat auf das DTO [`ObfuscatedBooking`] um (ACL, „innen → außen"), das
/// sie in den öffentlichen [`ObfuscatedStoreSeam`] staged (den io — rusqlite — erfüllt). Die vom Seam
/// **neu vergebene** ID hüllt sie „außen → innen" in die [`BookingId`] und reicht sie an den Handler
/// zurück, der genau sie als Zeiger publiziert; ein [`StoreUnavailable`] des Seams wird auf den
/// redigierten terminalen [`SourceError::StorageUnavailable`] gemappt — opake Quell-ID **plus** die
/// durchgereichte io-Diagnose (`reason`), damit ein unerreichbarer Store diagnostizierbar bleibt statt
/// an dieser Grenze zu verfallen. Kein Rohwert: die Redaktion der Diagnose schuldet der Seam.
pub(crate) struct RepositoryBridge<'a> {
    pub(crate) seam: &'a dyn ObfuscatedStoreSeam,
}

impl ObfuscatedRepository for RepositoryBridge<'_> {
    fn save(&self, booking: &DomainBooking) -> Result<BookingId, SourceError> {
        self.seam
            .save(&ObfuscatedBooking::from(booking))
            .map(BookingId::new)
            .map_err(
                |StoreUnavailable { message }| SourceError::StorageUnavailable {
                    source: booking.source_id().clone(),
                    reason: message,
                },
            )
    }
}
