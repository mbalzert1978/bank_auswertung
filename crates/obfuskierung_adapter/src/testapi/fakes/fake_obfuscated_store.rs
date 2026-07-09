use std::cell::RefCell;

use crate::abstractions::ObfuscatedStoreSeam;
use crate::dto::ObfuscatedBooking;

/// Fake der `obfuskiert`-Tabelle: hält die gestageten [`ObfuscatedBooking`]-DTOs im Speicher
/// (persistiert nichts auf Platte) und spielt so die vertagte Persistenz für die Specs. Beobachtet
/// wird das **Staging** — an den festgehaltenen DTOs prüft ein Spec die Feld-Treue der ganzen
/// obfuskierten Buchung am realen Durchlauf. Vorbild: der `FakeStoreSeam` des Einlese-Slice.
#[derive(Default)]
pub(crate) struct FakeObfuscatedStore {
    bookings: RefCell<Vec<ObfuscatedBooking>>,
}

impl FakeObfuscatedStore {
    /// Die gestageten obfuskierten Buchungen in Reihenfolge — das konkrete DTO am Seam.
    pub(crate) fn bookings(&self) -> Vec<ObfuscatedBooking> {
        self.bookings.borrow().clone()
    }
}

impl ObfuscatedStoreSeam for FakeObfuscatedStore {
    fn save(&self, booking: &ObfuscatedBooking) {
        self.bookings.borrow_mut().push(booking.clone());
    }
}
