use std::fmt;

/// Warum eine Rohzeile nicht zu einem gültigen `LineAggregate` wird. Jeder Fall ist
/// **terminal** — die Daten sind kaputt, ein erneuter Versuch hilft nicht; `io`s generische
/// `OutboxConsumer` mappt ihn später auf ein Dead-Letter, nicht auf Retry (nur echte
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

/// Warum die **Beschaffung** des Batches über den von der Domäne verlangten
/// [`RawLineSource`](crate::RawLineSource)-Port scheitert. Beide Fälle sind **terminal** —
/// ein Fehlschlag der Beschaffung ist nie Abwesenheit; der Adapter mappt sie auf ein
/// Dead-Letter, nicht auf Retry.
///
/// Die Domäne ist dependency-frei: `Display`/`Error` von Hand, kein `thiserror`.
/// **Datenschutz (Redaktion):** [`MissingMandatoryField`](SourceError::MissingMandatoryField)
/// ist **per Konstruktion** redigiert — es trägt nur den statischen Feldnamen, nie den
/// Rohwert; diese `Display` ist der **einzige Eigentümer** des Meldungssatzes (der Adapter
/// spiegelt ihn nicht; Kontrakt der Specs ist allein „der Grund enthält den Feldnamen").
/// [`Unavailable`](SourceError::Unavailable) trägt die vom Quell-Seam **durchgereichte
/// io-Diagnose** (Pfad, OS-/Parse-Fehler — Beschaffungs-Mechanik, nie Satz-/Feldinhalt;
/// diese Redaktions-Pflicht ist am Seam dokumentiert), damit der Dead-Letter-Grund benennt,
/// was genau schieflief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// Die Beschaffung selbst ist misslungen (io) — trägt die durchgereichte io-Diagnose.
    Unavailable { reason: String },
    /// Eine beschaffte Zeile ist da, aber ein **Pflichtfeld fehlt/ist leer** — redigiert
    /// per Konstruktion: nur der statische Feldname.
    MissingMandatoryField { field: &'static str },
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::Unavailable { reason } => write!(f, "source unavailable: {reason}"),
            SourceError::MissingMandatoryField { field } => {
                write!(f, "mandatory field missing: {field}")
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// Die **Fehlersumme** des [`ReadHandler`](crate::ReadHandler): entweder scheitert die
/// **Beschaffung** des Batches ([`SourceError`], Port `RawLineSource`) oder das **Deuten**
/// einer beschafften Zeile ([`LineError`]). Beide Äste sind terminal — der Adapter mappt
/// die Summe redigiert auf ein Dead-Letter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Die Beschaffung des Batches scheiterte (Beschaffungs- oder Präsenz-Fehler).
    Source(SourceError),
    /// Ein vorhandener Wert einer beschafften Zeile war nicht deutbar.
    Line(LineError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Source(error) => write!(f, "{error}"),
            ReadError::Line(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ReadError {
    /// Legt die gewrappte Ursache offen — repariert die Fehlerkette. Nur der innere Fehler
    /// wird sichtbar, dessen `Display` bereits redigiert ist (Feldname statt Rohwert); kein
    /// neuer Leak-Pfad.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadError::Source(error) => Some(error),
            ReadError::Line(error) => Some(error),
        }
    }
}

impl From<SourceError> for ReadError {
    fn from(error: SourceError) -> Self {
        ReadError::Source(error)
    }
}

impl From<LineError> for ReadError {
    fn from(error: LineError) -> Self {
        ReadError::Line(error)
    }
}
