use std::fmt;

/// Warum eine Rohzeile nicht zu einem gültigen `LineAggregate` wird. Jeder Fall ist
/// **terminal** — die Daten sind kaputt, ein erneuter Versuch hilft nicht; die
/// Consume-Bridge mappt ihn später auf ein Dead-Letter, nicht auf Retry (nur echte
/// Herkunfts-/IO-Fehler sind retryable, und die entstehen außerhalb der Domäne).
///
/// **Nur Dekodier-Fehler.** Die *Präsenz* eines Pflichtfelds prüft die Domäne nicht mehr —
/// sie **fordert** sie im Typ ([`RawLine`](crate::RawLine): Pflichtfeld non-`Option`); ein
/// fehlendes Pflichtfeld weist die Adapter-Abbildung ab, bevor eine `RawLine` entsteht.
/// Bleibt hier nur, was erst beim **Deuten** eines vorhandenen Werts scheitert (Betrag,
/// Währung, die zwei Kalenderdaten).
///
/// Die Domäne ist dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein
/// `thiserror`. **Datenschutz:** keine Variante trägt den rohen Feldinhalt. Gerade diese
/// „ungültig/unbekannt"-Fälle feuern, wenn ein Feld *nicht* das Erwartete enthält — bei
/// einem Spaltenverschub stünde sonst z. B. eine IBAN in einer (in ein Dead-Letter
/// persistierbaren) Meldung. Eine Meldung benennt darum nur Feld und Fehlerart, nie den
/// Inhalt; auch `Debug` kann so keinen Identifikator spiegeln. Für die zwei Datumsfelder
/// benennt sie zusätzlich, **welches** betroffen ist (Buchungstag vs. Valuta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineError {
    /// Der Betrag ist kein dekodierbarer Geldbetrag: der `Money`-Port konnte das Feld
    /// nicht deuten — inkl. eines fehlenden oder undeutbaren Vorzeichens.
    AmountInvalid,
    /// Der Währungscode ist keiner, den die Domäne kennt.
    CurrencyUnknown,
    /// Der Buchungstag ist kein gültiges Kalenderdatum.
    BookingDayInvalid,
    /// Die Valuta (Wertstellung) ist kein gültiges Kalenderdatum.
    ValueDateInvalid,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineError::AmountInvalid => write!(f, "not a decodable money amount"),
            LineError::CurrencyUnknown => write!(f, "unknown currency"),
            LineError::BookingDayInvalid => write!(f, "booking day is not a valid date"),
            LineError::ValueDateInvalid => write!(f, "value date is not a valid date"),
        }
    }
}

impl std::error::Error for LineError {}
