use crate::value_objects::NonEmptyString;

/// Die Währung einer Buchung — der Wert, auf dem die **eine-Währung**-Invariante des
/// [`AnalysisWindow`](crate::AnalysisWindow) sitzt (ADR-0005: die Domäne summiert keine gemischten
/// Währungen). Ein reiner Wrapper über den nicht-leeren Kern: die Gültigkeit liegt schon im
/// [`NonEmptyString`], hier gibt es nichts mehr abzuweisen.
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
