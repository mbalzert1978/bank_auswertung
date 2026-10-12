use crate::entities::ObfuscatedBooking;
use crate::ports::Obfuscator;
use crate::value_objects::{
    BookingId, KnownPersons, ObfuscatedCounterparty, PassThrough, RawCounterparty, Seed,
};

/// Das **rohe** Aggregat einer Buchung, wie der Handler es per ID aus dem einlesen-Speicher **lädt**
/// (der Lese-Port [`RawBookingSource`](crate::RawBookingSource) liefert es). Es trägt die zu
/// obfuskierende Gegenpartei, die durchzureichenden Felder und seine [`BookingId`] — die Quell-ID, die
/// als Fremdschlüssel in das obfuskierte Aggregat wandert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestedBooking {
    id: BookingId,
    counterparty: RawCounterparty,
    pass_through: PassThrough,
}

impl IngestedBooking {
    /// **Hydratisiert** das rohe Aggregat aus seinen bereits geprüften Teilen — kein Prägen einer
    /// frischen Identität, sondern Rekonstitution der von `einlesen` **schon vergebenen** Buchung: die
    /// Source-Bridge liest die Zeile per ID aus dem einlesen-Speicher und gießt die vom Adapter (ACL)
    /// gebauten Wert-Objekte hier hinein.
    #[must_use]
    pub fn hydrate(
        id: BookingId,
        counterparty: RawCounterparty,
        pass_through: PassThrough,
    ) -> Self {
        Self {
            id,
            counterparty,
            pass_through,
        }
    }

    /// **Obfuskiert** das Aggregat seed-basiert über den [`Obfuscator`]-Port: die Gegenpartei wird
    /// zu Token gedeutet (inkl. Fail-safe Firma/Person), ihr Freitext gegen die bekannten
    /// Personen-Namen gescrubbt ([`KnownPersons`]), Betrag/Datum &c. reisen unverändert mit,
    /// und der Seed wird ins Ergebnis mitpersistiert (reproduzierbar). Die Quell-ID reist als
    /// Fremdschlüssel mit; die neue Identität vergibt die Persistenz beim Ablegen.
    #[must_use]
    pub(crate) fn obfuscate(
        self,
        seed: Seed,
        obfuscator: &dyn Obfuscator,
        persons: &KnownPersons,
    ) -> ObfuscatedBooking {
        ObfuscatedBooking::new(
            self.id,
            ObfuscatedCounterparty::obfuscate(self.counterparty, seed, obfuscator, persons),
            self.pass_through,
            seed,
        )
    }
}
