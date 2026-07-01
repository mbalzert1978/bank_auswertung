/// Port für die Herkunft der Config: liefert den rohen Config-Text. Ob der aus
/// einer Datei, aus dem Speicher, aus dem Netz oder einem Test-Fake stammt, ist
/// austauschbar — dieser Crate hängt **nicht** an `std::fs`. Dependency
/// Inversion (ADR-0001): der Loader besitzt die Abstraktion, die er braucht,
/// nicht den konkreten Zugriff. Der konkrete Datei-Adapter lebt außerhalb (im
/// Composition Root).
pub trait Konfigquelle {
    /// Liefert den gesamten Config-Text — oder einen beliebigen Lesefehler.
    fn read(&self) -> Result<String, KonfigquelleError>;
}

/// Fehler einer [`Konfigquelle`] — bewusst offen für beliebige Herkunft
/// (Datei-I/O, Netz, …), damit die Abstraktion keine konkrete Fehlerquelle
/// vorschreibt.
pub type KonfigquelleError = Box<dyn std::error::Error + Send + Sync>;
