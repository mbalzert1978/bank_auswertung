/// Committet die anstehende Arbeitseinheit — die eingereihten Outbox-Nachrichten
/// **zusammen mit** den eigenen Zustandsänderungen des Aufrufers — **atomar** als
/// eine Transaktion (Outbox-Pattern). Der Prozessor committet eine Einheit je
/// Nachricht; der Adapter eines Producer-Slice committet seinen Zyklus. Beide
/// stagen auf einen Kontext und rufen dies **einmal** — daher `self` by value:
/// alles persistiert gemeinsam oder gar nicht.
///
/// **Vertrag, den die Typen nicht erzwingen (Implementierungs-Pflicht):** Die Schreib-
/// Ports der Einheit müssen die Arbeit **stagen** (nicht sofort dauerhaft schreiben),
/// und das **Fallenlassen ohne [`commit`](UnitOfWork::commit)** muss die gestagete
/// Arbeit **verwerfen** (Rollback). Nur so gilt „gemeinsam oder gar nicht" auch auf dem
/// Abbruch-Pfad: scheitert der Handler, wird die Einheit ungecommittet fallengelassen.
/// Rust kann diese zwei Eigenschaften nicht kompilierzeit-erzwingen — eine echte
/// `rusqlite::Transaction` erfüllt sie (Rollback bei Drop), der Test-Fake ebenso (eigene
/// `pending`-Puffer, nur bei `commit` geflusht).
pub trait UnitOfWork {
    /// Der Fehler, mit dem der Commit scheitern kann (Schranke `std::error::Error`,
    /// damit ihn jede Aufrufstelle ohne `where`-Wiederholung behandeln kann).
    type Error: std::error::Error;

    /// Committet die Arbeitseinheit atomar — oder scheitert mit [`Self::Error`].
    fn commit(self) -> Result<(), Self::Error>;
}
