use crate::value_objects::{BookingId, ObfuscatedCounterparty, PassThrough, Seed};

/// Das **obfuskierte** Aggregat — das Ergebnis, das der Handler ins Repository (die
/// `obfuskiert`-Tabelle) ablegt. Es referenziert die Quell-Buchung **nur per ID**
/// ([`source_id`](ObfuscatedBooking::source_id), Fremdschlüssel); seine **eigene** Identität vergibt
/// die Persistenz beim Ablegen. Trägt die obfuskierte Gegenpartei (nur Token), die durchgereichten
/// Felder und den mitpersistierten [`Seed`] (reproduzierbar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObfuscatedBooking {
    source_id: BookingId,
    counterparty: ObfuscatedCounterparty,
    pass_through: PassThrough,
    seed: Seed,
}

impl ObfuscatedBooking {
    #[must_use]
    pub(crate) fn new(
        source_id: BookingId,
        counterparty: ObfuscatedCounterparty,
        pass_through: PassThrough,
        seed: Seed,
    ) -> Self {
        Self {
            source_id,
            counterparty,
            pass_through,
            seed,
        }
    }

    #[must_use]
    pub fn source_id(&self) -> &BookingId {
        &self.source_id
    }

    #[must_use]
    pub fn counterparty(&self) -> &ObfuscatedCounterparty {
        &self.counterparty
    }

    #[must_use]
    pub fn pass_through(&self) -> &PassThrough {
        &self.pass_through
    }

    #[must_use]
    pub fn seed(&self) -> Seed {
        self.seed
    }
}
