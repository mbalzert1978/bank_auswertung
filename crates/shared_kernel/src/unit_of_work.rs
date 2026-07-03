/// Committet die anstehende Arbeitseinheit — die eingereihten Outbox-Nachrichten
/// **zusammen mit** den eigenen Zustandsänderungen des Aufrufers — **atomar** als
/// eine Transaktion (Outbox-Pattern). Der Prozessor committet eine Einheit je
/// Nachricht; der Adapter eines Producer-Slice committet seinen Zyklus. Beide
/// stagen auf einen Kontext und rufen dies **einmal** — daher `self` by value:
/// alles persistiert gemeinsam oder gar nicht.
pub trait UnitOfWork {
    /// Der Fehler, mit dem der Commit scheitern kann (Schranke `std::error::Error`,
    /// damit ihn jede Aufrufstelle ohne `where`-Wiederholung behandeln kann).
    type Error: std::error::Error;

    /// Committet die Arbeitseinheit atomar — oder scheitert mit [`Self::Error`].
    fn commit(self) -> Result<(), Self::Error>;
}
