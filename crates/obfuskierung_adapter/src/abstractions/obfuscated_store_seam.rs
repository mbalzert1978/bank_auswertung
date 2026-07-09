use crate::dto::ObfuscatedBooking;

/// Der öffentliche **Speicher-Seam** der `obfuskiert`-Tabelle: legt (staged) eine obfuskierte
/// Buchung ab. **io erfüllt ihn** (sqlx) — die Tabelle ist die Wahrheit für stromabwärts. Beobachtet
/// wird das **Staging**; den Commit (Tabelle **und** die `Obfuscated`-Outbox-Nachricht atomar in
/// **einer** `UnitOfWork`) besorgt io. Vorbild: der `StoreSeam` des Einlese-Slice.
pub trait ObfuscatedStoreSeam {
    /// Reiht die obfuskierte Buchung in die offene Arbeitseinheit ein (Staging).
    fn save(&self, booking: &ObfuscatedBooking);
}
