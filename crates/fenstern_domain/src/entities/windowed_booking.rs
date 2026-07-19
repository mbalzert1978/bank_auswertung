use crate::value_objects::{BookingId, Currency};

/// Eine **obfuskierte Buchung der Seite**, wie der Lese-Port
/// [`ObfuscatedWindowSource`](crate::ObfuscatedWindowSource) sie liefert — genau das, was das
/// Fenster von ihr braucht: ihre [`BookingId`] (die es als Fremdschlüssel einfriert) und ihre
/// [`Currency`] (auf der die eine-Währung-Invariante sitzt). Alles Übrige bleibt in der
/// `obfuskiert`-Ablage; das Fenster referenziert Fremd-Entitäten **nur per ID** (ADR-0012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowedBooking {
    id: BookingId,
    currency: Currency,
}

impl WindowedBooking {
    /// **Hydratisiert** die Zeile aus ihren bereits geprüften Teilen — keine frische Identität,
    /// sondern Rekonstitution der von `obfuskieren` schon vergebenen Buchung. Die Präsenz ihrer
    /// Pflichtfelder erzwingt der Adapter (ACL), bevor sie hier ankommt.
    #[must_use]
    pub fn hydrate(id: BookingId, currency: Currency) -> Self {
        Self { id, currency }
    }

    pub(crate) fn id(&self) -> &BookingId {
        &self.id
    }

    pub(crate) fn currency(&self) -> &Currency {
        &self.currency
    }
}
