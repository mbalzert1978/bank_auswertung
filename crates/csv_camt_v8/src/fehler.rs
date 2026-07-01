use eigene_konten::IbanError;

use crate::port::CsvQuelleError;

/// Warum ein CSV-Import fehlschlug. Jeder Fehler bricht den Import laut ab — kein
/// stillschweigendes Überspringen einer Zeile und keine Falschsummierung.
///
/// Datenschutz: die Meldungen enthalten **keine** rohe IBAN/BIC/Kontonummer. Ein
/// struktureller Identifikator soll auch nicht über eine Fehlermeldung (Log) die
/// Maschine verlassen; das konkrete Auftragskonto ist für die Behebung nicht
/// nötig (der Nutzer kennt seine Config).
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// Die CSV-Quelle konnte nicht gelesen werden (z. B. Datei-I/O).
    #[error("CSV-Quelle konnte nicht gelesen werden: {0}")]
    Source(#[source] CsvQuelleError),
    /// Das CSV ist strukturell kaputt (falsche Spaltenzahl, fehlende Kopfzeile …).
    #[error("CSV konnte nicht geparst werden: {0}")]
    Csv(#[from] csv::Error),
    /// Das `Auftragskonto` ist keine strukturell gültige IBAN.
    #[error("Auftragskonto ist keine gültige IBAN: {0}")]
    AuftragskontoInvalid(#[source] IbanError),
    /// Das `Auftragskonto` ist nicht in der Eigene-Konten-Config hinterlegt —
    /// vermutlich ein CSV für ein Konto, das noch nicht konfiguriert ist.
    #[error("ein Auftragskonto ist nicht in der Eigene-Konten-Config hinterlegt")]
    AuftragskontoUnknown,
    /// Der Betrag ist kein gültiger deutscher Geldbetrag (`-1.234,56`).
    #[error("Betrag ist kein gültiger deutscher Geldbetrag: {0}")]
    BetragInvalid(String),
    /// Weder eine Gläubiger-ID noch ein Empfänger-Name — die Gegenpartei ist
    /// nicht identifizierbar.
    #[error("Empfänger fehlt: weder Gläubiger-ID noch Name vorhanden")]
    EmpfängerMissing,
    /// Die Datei enthält mehr als eine Währung — die Domäne summiert keine
    /// gemischten Währungen.
    #[error("uneinheitliche Währung in der Datei: {1} weicht von {0} ab")]
    MixedWährung(String, String),
    /// Eine Währung, die die Domäne (noch) nicht kennt.
    #[error("unbekannte Währung: {0}")]
    UnknownWährung(String),
}
