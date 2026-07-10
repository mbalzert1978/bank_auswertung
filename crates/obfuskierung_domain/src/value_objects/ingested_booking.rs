use crate::ports::Obfuscator;
use crate::value_objects::booking_id::BookingId;
use crate::value_objects::obfuscated_booking::ObfuscatedBooking;
use crate::value_objects::obfuscated_counterparty::ObfuscatedCounterparty;
use crate::value_objects::pass_through::PassThrough;
use crate::value_objects::raw_counterparty::RawCounterparty;
use crate::value_objects::seed::Seed;

/// Das **rohe** Aggregat einer Buchung, wie der Handler es per ID aus dem einlesen-Speicher **lädt**
/// (der Lese-Port [`RawBookingSource`](crate::RawBookingSource) liefert es; das IO dahinter — die
/// Roh-Tabelle aus [0015](../../../docs/issues/0015-einlesen-slice-als-pipeline-consumer.md) — ist
/// vertagt). Es trägt die zu obfuskierende Gegenpartei und die durchzureichenden Felder plus seine
/// Identität.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestedBooking {
    id: BookingId,
    counterparty: RawCounterparty,
    pass_through: PassThrough,
}

impl IngestedBooking {
    /// Baut das rohe Aggregat aus seinen Teilen — die Source-Bridge setzt es aus der gelesenen
    /// einlesen-Zeile zusammen.
    pub fn new(id: BookingId, counterparty: RawCounterparty, pass_through: PassThrough) -> Self {
        Self {
            id,
            counterparty,
            pass_through,
        }
    }

    /// **Obfuskiert** das Aggregat seed-basiert über den [`Obfuscator`]-Port: die Gegenpartei wird
    /// zu Token gedeutet (inkl. Fail-safe Firma/Person), Betrag/Datum &c. reisen unverändert mit,
    /// und der Seed wird ins Ergebnis mitpersistiert (reproduzierbar). Identität bleibt erhalten.
    pub fn obfuscate(self, seed: Seed, obfuscator: &dyn Obfuscator) -> ObfuscatedBooking {
        ObfuscatedBooking::new(
            self.id,
            ObfuscatedCounterparty::obfuscate(self.counterparty, seed, obfuscator),
            self.pass_through,
            seed,
        )
    }
}
