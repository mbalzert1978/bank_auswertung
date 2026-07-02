use crate::buchung::Buchung;
use crate::fehler::ImportError;
use crate::zeile::Umsatzzeile;

/// Der `Umsatzquelle`-Port (inbound, ADR-0001): die Anwendung bezieht ihre
/// Buchungen ausschließlich hierüber — bereits obfuskiert, als `Buchung`-DTOs.
/// Welches Dateiformat dahintersteckt (CSV-CAMT-v8 zuerst, XML-camt/MT940 später),
/// bleibt austauschbar: ein neues Format ist ein neuer Adapter hinter diesem Port.
pub trait Umsatzquelle {
    /// Liefert alle Buchungen der Quelle — bereits obfuskiert — oder einen
    /// Importfehler. Kein stillschweigendes Überspringen einer Zeile.
    fn buchungen(&self) -> Result<Vec<Buchung>, ImportError>;
}

/// Der public Seam für die Herkunft der bereits **geparsten** CSV-Zeilen: liefert
/// je Datenzeile eine [`Umsatzzeile`] mit den Rohfeldern. **IO** erfüllt diesen
/// Seam (liest die Datei, dekodiert nach UTF-8, parst das CSV mit Delimiter/Quoting
/// über die CAMT-v8-Kopfzeile und baut die DTOs); dieser Adapter-Crate hängt damit
/// an keiner Datei- und keiner CSV-Bibliothek
/// ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
pub trait Zeilenquelle {
    /// Liefert die geparsten Zeilen — oder einen beliebigen Herkunfts-/Formatfehler.
    fn zeilen(&self) -> Result<Vec<Umsatzzeile>, ZeilenquelleError>;
}

/// Fehler einer [`Zeilenquelle`] — bewusst offen für beliebige Herkunft (Datei-I/O,
/// CSV-Parse, Zeichensatz), damit die Abstraktion keine konkrete Quelle vorschreibt.
pub type ZeilenquelleError = Box<dyn std::error::Error + Send + Sync>;
