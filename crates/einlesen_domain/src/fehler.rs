use std::fmt;

/// Warum eine Rohzeile nicht zu einem gültigen `ZeilenAggregat` wird. Jeder Fall ist
/// **terminal** — die Daten sind kaputt, ein erneuter Versuch hilft nicht; die
/// Consume-Bridge mappt ihn später auf ein Dead-Letter, nicht auf Retry (nur echte
/// Herkunfts-/IO-Fehler sind retryable, und die entstehen außerhalb der Domäne).
///
/// Die Domäne ist dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein
/// `thiserror`. **Datenschutz:** keine Variante trägt den rohen Feldinhalt. Gerade die
/// „ungültig/unbekannt"-Fälle feuern, wenn ein Feld *nicht* das Erwartete enthält — bei
/// einem Spaltenverschub stünde sonst z. B. eine IBAN in einer (in ein Dead-Letter
/// persistierbaren) Meldung. Eine Meldung benennt darum nur Feld und Fehlerart, nie den
/// Inhalt; auch `Debug` kann so keinen Identifikator spiegeln. Für die zwei Datumsfelder
/// benennt sie zusätzlich, **welches** betroffen ist (Buchungstag vs. Valuta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZeilenError {
    /// Der Betrag ist kein gültiger deutscher Geldbetrag.
    BetragUngültig,
    /// Der Betrag fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    BetragFehlt,
    /// Das Betragsfeld trägt kein führendes Vorzeichen — die Richtung ist nicht deutbar.
    VorzeichenFehlt,
    /// Der Währungscode ist keiner, den die Domäne kennt.
    WährungUnbekannt,
    /// Die Währung fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    WährungFehlt,
    /// Das Auftragskonto fehlt — ohne eigenes Konto ist die Zeile nicht zuordenbar.
    AuftragskontoFehlt,
    /// Weder Gläubiger-ID noch Name — die Gegenpartei ist nicht identifizierbar.
    EmpfängerFehlt,
    /// Der Buchungstag fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    BuchungstagFehlt,
    /// Der Buchungstag ist kein gültiges Kalenderdatum.
    BuchungstagUngültig,
    /// Die Valuta (Wertstellung) fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    ValutaFehlt,
    /// Die Valuta (Wertstellung) ist kein gültiges Kalenderdatum.
    ValutaUngültig,
}

impl fmt::Display for ZeilenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZeilenError::BetragUngültig => write!(f, "kein gültiger deutscher Geldbetrag"),
            ZeilenError::BetragFehlt => write!(f, "Betrag fehlt"),
            ZeilenError::VorzeichenFehlt => {
                write!(f, "Vorzeichen fehlt: Richtung nicht deutbar")
            }
            ZeilenError::WährungUnbekannt => write!(f, "unbekannte Währung"),
            ZeilenError::WährungFehlt => write!(f, "Währung fehlt"),
            ZeilenError::AuftragskontoFehlt => write!(f, "Auftragskonto fehlt"),
            ZeilenError::EmpfängerFehlt => {
                write!(f, "Empfänger fehlt: weder Gläubiger-ID noch Name vorhanden")
            }
            ZeilenError::BuchungstagFehlt => write!(f, "Buchungstag fehlt"),
            ZeilenError::BuchungstagUngültig => {
                write!(f, "Buchungstag ist kein gültiges Datum")
            }
            ZeilenError::ValutaFehlt => write!(f, "Valuta fehlt"),
            ZeilenError::ValutaUngültig => write!(f, "Valuta ist kein gültiges Datum"),
        }
    }
}

impl std::error::Error for ZeilenError {}
