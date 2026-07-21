use std::fmt;

/// Warum eine Rohzeile nicht zu einem gültigen `LineAggregate` wird. Jeder Fall ist
/// **terminal** — die Daten sind kaputt, ein erneuter Versuch hilft nicht.
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
/// einem Spaltenverschub stünde sonst z. B. eine IBAN in einer (lokal persistierbaren)
/// Meldung. Eine Meldung benennt darum nur Feld und Fehlerart, nie den Inhalt; auch `Debug`
/// kann so keinen Identifikator spiegeln. Für die zwei Datumsfelder benennt sie zusätzlich,
/// **welches** betroffen ist (Buchungstag vs. Valuta).
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
/// ein Fehlschlag der Beschaffung ist nie Abwesenheit.
///
/// Die Domäne ist dependency-frei: `Display`/`Error` von Hand, kein `thiserror`.
/// **Datenschutz (Redaktion):** [`MissingMandatoryField`](SourceError::MissingMandatoryField)
/// ist **per Konstruktion** redigiert — es trägt nur den statischen Feldnamen, nie den
/// Rohwert; diese `Display` ist der **einzige Eigentümer** des Meldungssatzes (der Adapter
/// spiegelt ihn nicht; Kontrakt der Specs ist allein „der Grund enthält den Feldnamen").
/// [`MixedCurrency`](SourceError::MixedCurrency) ist **datenlos** und damit per Konstruktion
/// redigiert — es nennt nur die Mechanik, nie einen Währungscode (auch `Debug` spiegelt keinen).
/// [`Unavailable`](SourceError::Unavailable) trägt die vom Quell-Seam **durchgereichte
/// io-Diagnose** (Pfad, OS-/Parse-Fehler — Beschaffungs-Mechanik, nie Satz-/Feldinhalt;
/// diese Redaktions-Pflicht ist am Seam dokumentiert), damit der terminale Fehlergrund
/// benennt, was genau schieflief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// Die Beschaffung selbst ist misslungen (io) — trägt die durchgereichte io-Diagnose.
    Unavailable { reason: String },
    /// Eine beschaffte Zeile ist da, aber ein **Pflichtfeld fehlt/ist leer** — redigiert
    /// per Konstruktion: nur der statische Feldname.
    MissingMandatoryField { field: &'static str },
    /// Der beschaffte Batch führt **mehr als eine** Währung — die Domäne summiert keine
    /// gemischten Währungen, darum eine **formatunabhängige** Inbound-Zusicherung der ACL
    /// (nicht des CSV-Readers). Redigiert per Konstruktion: **datenlos**, nie ein Code.
    MixedCurrency,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::Unavailable { reason } => write!(f, "source unavailable: {reason}"),
            SourceError::MissingMandatoryField { field } => {
                write!(f, "mandatory field missing: {field}")
            }
            SourceError::MixedCurrency => {
                write!(f, "source is not single-currency across the batch")
            }
        }
    }
}

impl std::error::Error for SourceError {}

/// Warum ein **Egress-Schritt** einer gültigen Zeile terminal scheitert: das **Ablegen** des
/// Aggregats (das die Identität vergibt) oder das **Veröffentlichen** des
/// „Eingelesen"-Ereignisses. Beide sind io-Zugriffe (Persistenz-/Enqueue-Mechanik) und
/// **terminal** — ein erneuter Versuch im Slice findet nicht statt.
///
/// Die Domäne ist dependency-frei: `Display`/`Error` von Hand, kein `thiserror`.
/// **Datenschutz:** der `reason` trägt die vom jeweiligen Seam **durchgereichte io-Diagnose**
/// (Zugriffs-Mechanik, nie Satz-/Feldinhalt; die Redaktions-Pflicht ist am Seam dokumentiert).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkError {
    /// Das Ablegen des Aggregats ist misslungen — trägt die durchgereichte io-Diagnose.
    StoreFailed { reason: String },
    /// Das Veröffentlichen des Ereignisses ist misslungen — trägt die durchgereichte io-Diagnose.
    PublishFailed { reason: String },
}

impl fmt::Display for SinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SinkError::StoreFailed { reason } => write!(f, "store failed: {reason}"),
            SinkError::PublishFailed { reason } => write!(f, "publish failed: {reason}"),
        }
    }
}

impl std::error::Error for SinkError {}

/// Die **Fehlersumme** des [`ReadHandler`](crate::ReadHandler): die **Beschaffung** des Batches
/// ([`SourceError`]), das **Deuten** einer beschafften Zeile ([`LineError`]) oder das
/// **Ablegen/Veröffentlichen** einer gültigen Zeile ([`SinkError`]) scheitert. Alle Äste sind
/// **terminal**; der Handler gibt seinen nackten, redigierten Fehler zurück.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Die Beschaffung des Batches scheiterte (Beschaffungs- oder Präsenz-Fehler).
    Source(SourceError),
    /// Ein vorhandener Wert einer beschafften Zeile war nicht deutbar.
    Line(LineError),
    /// Das Ablegen oder Veröffentlichen einer gültigen Zeile scheiterte.
    Sink(SinkError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Source(error) => write!(f, "{error}"),
            ReadError::Line(error) => write!(f, "{error}"),
            ReadError::Sink(error) => write!(f, "{error}"),
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
            ReadError::Sink(error) => Some(error),
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

impl From<SinkError> for ReadError {
    fn from(error: SinkError) -> Self {
        ReadError::Sink(error)
    }
}
