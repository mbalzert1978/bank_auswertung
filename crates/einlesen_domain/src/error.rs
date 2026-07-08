use std::fmt;

/// Warum eine Rohzeile nicht zu einem gültigen `LineAggregate` wird. Jeder Fall ist
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
pub enum LineError {
    /// Der Betrag ist kein gültiger deutscher Geldbetrag.
    AmountInvalid,
    /// Der Betrag fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    AmountMissing,
    /// Das Betragsfeld trägt kein führendes Vorzeichen — die Richtung ist nicht deutbar.
    SignMissing,
    /// Der Währungscode ist keiner, den die Domäne kennt.
    CurrencyUnknown,
    /// Die Währung fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    CurrencyMissing,
    /// Das Auftragskonto fehlt — ohne eigenes Konto ist die Zeile nicht zuordenbar.
    OrderingAccountMissing,
    /// Weder Gläubiger-ID noch Name — die Gegenpartei ist nicht identifizierbar.
    RecipientMissing,
    /// Der Buchungstag fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    BookingDayMissing,
    /// Der Buchungstag ist kein gültiges Kalenderdatum.
    BookingDayInvalid,
    /// Die Valuta (Wertstellung) fehlt ganz (Pflichtfeld leer/nicht vorhanden).
    ValueDateMissing,
    /// Die Valuta (Wertstellung) ist kein gültiges Kalenderdatum.
    ValueDateInvalid,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineError::AmountInvalid => write!(f, "not a valid German money amount"),
            LineError::AmountMissing => write!(f, "amount missing"),
            LineError::SignMissing => write!(f, "sign missing: direction cannot be inferred"),
            LineError::CurrencyUnknown => write!(f, "unknown currency"),
            LineError::CurrencyMissing => write!(f, "currency missing"),
            LineError::OrderingAccountMissing => write!(f, "ordering account missing"),
            LineError::RecipientMissing => {
                write!(f, "recipient missing: neither creditor id nor name present")
            }
            LineError::BookingDayMissing => write!(f, "booking day missing"),
            LineError::BookingDayInvalid => write!(f, "booking day is not a valid date"),
            LineError::ValueDateMissing => write!(f, "value date missing"),
            LineError::ValueDateInvalid => write!(f, "value date is not a valid date"),
        }
    }
}

impl std::error::Error for LineError {}
