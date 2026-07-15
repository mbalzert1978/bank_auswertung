use super::NonEmptyString;

/// Die **maschinelle Art** einer Buchung (`LASTSCHRIFT`, `DAUERAUFTRAG`,
/// `ÜBERWEISUNG`, `ENTGELT` …, CONTEXT.md) — hier **roh** getragen. Ihre Deutung
/// (Gebühr? wiederkehrend?) ist Treiber der deterministischen Erkennung und damit
/// Sache des `auswerten`-Slice, **nicht** dieser Stufe: `einlesen` klassifiziert
/// nicht, es reicht den Code weiter. Im Aggregat optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingText(NonEmptyString);

impl From<NonEmptyString> for BookingText {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

impl BookingText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
