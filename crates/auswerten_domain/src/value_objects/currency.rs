use crate::value_objects::NonEmptyString;

/// Die **eine** Währung eines Auswertungsfensters. Sie steht **einmal** am Aggregat, nicht an jedem
/// Betrag: dass alle Buchungen eines Fensters dieselbe Währung tragen, hat `fenstern` schon **an der
/// Konstruktion** des Fensters erzwungen (ADR-0013, Entscheidung A) — dieser Slice liest den Beweis,
/// er führt ihn nicht erneut.
///
/// Reiner Wrapper über den nicht-leeren Kern: die Gültigkeit liegt schon im [`NonEmptyString`], hier
/// gibt es nichts mehr abzuweisen.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Currency(NonEmptyString);

impl Currency {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<NonEmptyString> for Currency {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}
