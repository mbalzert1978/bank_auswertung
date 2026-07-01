use domain::Buchung;

use crate::fehler::ImportError;

/// Der `Umsatzquelle`-Port (inbound, ADR-0001): die Anwendung bezieht ihre
/// Buchungen ausschließlich hierüber. Welches Dateiformat dahintersteckt
/// (CSV-CAMT-v8 zuerst, XML-camt/MT940 später), bleibt austauschbar — ein neues
/// Format ist ein neuer Adapter hinter diesem Port, keine Domänenänderung.
pub trait Umsatzquelle {
    /// Liefert alle Buchungen der Quelle — bereits obfuskiert — oder einen
    /// Importfehler. Kein stillschweigendes Überspringen einer Zeile.
    fn buchungen(&self) -> Result<Vec<Buchung>, ImportError>;
}

/// Herkunft des rohen CSV-Texts. Hält den Adapter `std::fs`-frei (Dependency
/// Inversion, ADR-0001): ob Datei, Speicher oder Test-Fake ist austauschbar; der
/// konkrete Datei-Zugriff lebt außerhalb (Composition Root) — analog zu
/// [`eigene_konten::Konfigquelle`]. Der gelieferte Text ist bereits nach UTF-8
/// dekodiert.
pub trait CsvQuelle {
    /// Liefert den gesamten CSV-Text — oder einen beliebigen Lesefehler.
    fn read(&self) -> Result<String, CsvQuelleError>;
}

/// Fehler einer [`CsvQuelle`] — bewusst offen für beliebige Herkunft (Datei-I/O,
/// Netz, …), damit die Abstraktion keine konkrete Fehlerquelle vorschreibt.
pub type CsvQuelleError = Box<dyn std::error::Error + Send + Sync>;
