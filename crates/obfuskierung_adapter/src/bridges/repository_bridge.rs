use obfuskierung_domain::{ObfuscatedBooking as DomainBooking, ObfuscatedRepository};

use crate::abstractions::ObfuscatedStoreSeam;
use crate::dto::ObfuscatedBooking;

/// Die **Port-Bridge** für die Ablage: erfüllt `obfuskierung_domain::ObfuscatedRepository` und setzt
/// das obfuskierte Domänen-Aggregat auf das DTO [`ObfuscatedBooking`] um (ACL, „innen → außen"), das
/// sie in den öffentlichen [`ObfuscatedStoreSeam`] staged (den io — sqlx — erfüllt). Vorbild: der
/// `StoreBridge` des Einlese-Slice.
pub(crate) struct RepositoryBridge<'a> {
    pub(crate) seam: &'a dyn ObfuscatedStoreSeam,
}

impl ObfuscatedRepository for RepositoryBridge<'_> {
    fn save(&self, booking: &DomainBooking) {
        self.seam.save(&ObfuscatedBooking::from(booking));
    }
}
