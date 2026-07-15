use obfuskierung_domain::{
    BookingId, ObfuscatedBooking as DomainBooking, ObfuscatedRepository, SourceError,
};

use crate::abstractions::ObfuscatedStoreSeam;
use crate::models::ObfuscatedBooking;

/// Die **Port-Bridge** für die Ablage: erfüllt `obfuskierung_domain::ObfuscatedRepository` und setzt
/// das obfuskierte Domänen-Aggregat auf das DTO [`ObfuscatedBooking`] um (ACL, „innen → außen"), das
/// sie in den öffentlichen [`ObfuscatedStoreSeam`] staged (den io — sqlx — erfüllt). Die vom Seam
/// **neu vergebene** ID hüllt sie „außen → innen" in die [`BookingId`] und reicht sie an den Handler
/// zurück, der genau sie als Zeiger publiziert; ein
/// [`StoreUnavailable`](crate::abstractions::StoreUnavailable) des Seams wird auf den redigierten
/// terminalen [`SourceError::StorageUnavailable`] gemappt (opake Quell-ID, kein Rohwert).
pub(crate) struct RepositoryBridge<'a> {
    pub(crate) seam: &'a dyn ObfuscatedStoreSeam,
}

impl ObfuscatedRepository for RepositoryBridge<'_> {
    fn save(&self, booking: &DomainBooking) -> Result<BookingId, SourceError> {
        match self.seam.save(&ObfuscatedBooking::from(booking)) {
            Ok(id) => Ok(BookingId::new(id)),
            Err(_) => Err(SourceError::StorageUnavailable(booking.source_id().clone())),
        }
    }
}
