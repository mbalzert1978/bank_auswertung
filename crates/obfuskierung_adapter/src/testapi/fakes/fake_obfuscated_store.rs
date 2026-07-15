use std::cell::RefCell;

use crate::abstractions::{ObfuscatedStoreSeam, StoreUnavailable};
use crate::models::ObfuscatedBooking;

/// Fake der `obfuskiert`-Tabelle: hält die gestageten [`ObfuscatedBooking`]-DTOs im Speicher
/// (persistiert nichts auf Platte) und spielt so die vertagte Persistenz für die Specs. Beobachtet
/// wird das **Staging** — an den festgehaltenen DTOs prüft ein Spec die Feld-Treue der ganzen
/// obfuskierten Buchung am realen Durchlauf.
///
/// **Die vergebene ID** ist deterministisch (`obfuskiert-1`, `obfuskiert-2`, … in Staging-Reihenfolge,
/// wie der `einlesen`-Fake) — der Fake spielt `INSERT … RETURNING id`, damit ein Spec den publizierten
/// Zeiger exakt gegen die neu vergebene ID prüfen kann.
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
    fn save(&self, booking: &ObfuscatedBooking) -> Result<String, StoreUnavailable> {
        let mut bookings = self.bookings.borrow_mut();
        let id = format!("obfuskiert-{}", bookings.len() + 1);
        bookings.push(booking.clone());
        Ok(id)
    }
}
