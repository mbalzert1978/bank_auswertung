use super::NonEmptyString;

/// Der Freitext einer Buchung — zweitwichtigstes Kategorie-Signal nach dem Empfänger
/// (CONTEXT.md). Roh getragen; im Aggregat optional (nicht jede Buchung führt einen).
/// Das best-effort-Scrubben eingebetteter Identifikatoren ist Sache des
/// `obfuskieren`-Slice, nicht dieser Stufe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verwendungszweck(NonEmptyString);

impl From<NonEmptyString> for Verwendungszweck {
    fn from(wert: NonEmptyString) -> Self {
        Self(wert)
    }
}

impl Verwendungszweck {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
