use crate::value_objects::booking_id::BookingId;
use crate::value_objects::obfuscated_counterparty::ObfuscatedCounterparty;
use crate::value_objects::pass_through::PassThrough;
use crate::value_objects::seed::Seed;

/// Das **obfuskierte** Aggregat — das Ergebnis, das der Handler ins Repository (die
/// `obfuskiert`-Tabelle) ablegt und dessen [`id`](ObfuscatedBooking::id) er als Zeiger publiziert
/// (Tabelle = Wahrheit, Message = Pointer). Trägt die obfuskierte Gegenpartei (nur Token), die
/// durchgereichten Felder, den mitpersistierten [`Seed`] (reproduzierbar) und seine Identität.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObfuscatedBooking {
    id: BookingId,
    counterparty: ObfuscatedCounterparty,
    pass_through: PassThrough,
    seed: Seed,
}

impl ObfuscatedBooking {
    pub(crate) fn new(
        id: BookingId,
        counterparty: ObfuscatedCounterparty,
        pass_through: PassThrough,
        seed: Seed,
    ) -> Self {
        Self {
            id,
            counterparty,
            pass_through,
            seed,
        }
    }

    pub fn id(&self) -> &BookingId {
        &self.id
    }

    pub fn counterparty(&self) -> &ObfuscatedCounterparty {
        &self.counterparty
    }

    pub fn pass_through(&self) -> &PassThrough {
        &self.pass_through
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }
}
