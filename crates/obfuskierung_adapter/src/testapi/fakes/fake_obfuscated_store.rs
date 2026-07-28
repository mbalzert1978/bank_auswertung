use std::cell::RefCell;

use super::ArrangedFailure;
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
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Ablage-Fehlschlag — wie die
/// Fakes der drei anderen Slices.
#[derive(Default)]
pub(crate) struct FakeObfuscatedStore {
    bookings: RefCell<Vec<ObfuscatedBooking>>,
    failure: ArrangedFailure,
}

impl FakeObfuscatedStore {
    /// Die gestageten obfuskierten Buchungen in Reihenfolge — das konkrete DTO am Seam.
    pub(crate) fn bookings(&self) -> Vec<ObfuscatedBooking> {
        self.bookings.borrow().clone()
    }

    /// Lässt jede Ablage terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure.arm(message);
    }
}

impl ObfuscatedStoreSeam for FakeObfuscatedStore {
    fn save(&self, booking: &ObfuscatedBooking) -> Result<String, StoreUnavailable> {
        self.failure.check(|message| StoreUnavailable { message })?;
        let mut bookings = self.bookings.borrow_mut();
        let id = format!("obfuskiert-{}", bookings.len() + 1);
        bookings.push(booking.clone());
        Ok(id)
    }
}
