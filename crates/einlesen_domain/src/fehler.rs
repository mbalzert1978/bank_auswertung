use std::fmt;

/// Warum eine Rohzeile nicht zu einem gültigen `ZeilenAggregat` wird. Jeder Fall ist
/// **terminal** — die Daten sind kaputt, ein erneuter Versuch hilft nicht; die
/// Consume-Bridge mappt ihn später auf ein Dead-Letter, nicht auf Retry (nur echte
/// Herkunfts-/IO-Fehler sind retryable, und die entstehen außerhalb der Domäne).
///
/// Die Domäne ist dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein
/// `thiserror`. Datenschutz: die Meldungen tragen nur den rohen Betrag, Währungscode
/// bzw. das rohe Datum — **kein** struktureller Identifikator (Auftragskonto o. Ä.)
/// steht je in einer Meldung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZeilenError {
    /// Der Betrag ist kein gültiger deutscher Geldbetrag (`-1.234,56`).
    BetragUngültig(String),
    /// Der Währungscode ist keiner, den die Domäne kennt.
    WährungUnbekannt(String),
    /// Das Auftragskonto fehlt — ohne eigenes Konto ist die Zeile nicht zuordenbar.
    AuftragskontoFehlt,
    /// Weder Gläubiger-ID noch Name — die Gegenpartei ist nicht identifizierbar.
    EmpfängerFehlt,
    /// Der rohe Datumswert ergibt kein gültiges Kalenderdatum.
    DatumUngültig(String),
    /// Die Währung fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    WährungFehlt,
    /// Der Betrag fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    BetragFehlt,
    /// Das Betragsfeld trägt kein führendes Vorzeichen — die Richtung ist nicht deutbar.
    VorzeichenFehlt,
    /// Ein Datum (Buchungstag oder Valuta) fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    DatumFehlt,
}

impl fmt::Display for ZeilenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZeilenError::BetragUngültig(roh) => {
                write!(f, "kein gültiger deutscher Geldbetrag: {roh}")
            }
            ZeilenError::WährungUnbekannt(code) => write!(f, "unbekannte Währung: {code}"),
            ZeilenError::AuftragskontoFehlt => write!(f, "Auftragskonto fehlt"),
            ZeilenError::EmpfängerFehlt => {
                write!(f, "Empfänger fehlt: weder Gläubiger-ID noch Name vorhanden")
            }
            ZeilenError::DatumUngültig(roh) => write!(f, "kein gültiges Datum: {roh}"),
            ZeilenError::WährungFehlt => write!(f, "Währung fehlt"),
            ZeilenError::BetragFehlt => write!(f, "Betrag fehlt"),
            ZeilenError::VorzeichenFehlt => {
                write!(f, "Vorzeichen fehlt: Richtung nicht deutbar")
            }
            ZeilenError::DatumFehlt => write!(f, "Datum fehlt (Buchungstag oder Valuta)"),
        }
    }
}

impl std::error::Error for ZeilenError {}
