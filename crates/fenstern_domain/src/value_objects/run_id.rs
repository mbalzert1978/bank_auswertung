use super::NonEmptyString;

/// Die **Identität eines Analyse-Laufs**
/// ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)): jedes
/// Auswertungsfenster gehört zu genau **einem** Lauf, und die fenster-übergreifende Reichweite von
/// `auswerten` sieht nur die Fenster ihres Laufs. Ohne sie zählten die Fenster jedes früheren Laufs
/// über dieselbe Historie erneut mit — jeder Lauf paginiert ja die **ganze** obfuskierte Historie.
///
/// **Opak.** Vergeben wird sie dort, wo der Lauf beginnt — beim Anstoß, außerhalb des Slice; der
/// Slice reicht sie unverändert vom [`StartAnalysis`](crate::StartAnalysis) durch jedes
/// [`AdvanceWindow`](crate::AdvanceWindow) bis in jedes Fenster und den Abschluss. Er deutet sie nie.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunId(NonEmptyString);

impl From<NonEmptyString> for RunId {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

impl RunId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
