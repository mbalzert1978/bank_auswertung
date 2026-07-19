/// Die **eigene** Identität eines abgelegten [`AnalysisWindow`](crate::AnalysisWindow) — die
/// **Ablage vergibt sie** (io: `INSERT … RETURNING id`), und **genau sie** publiziert der Slice als
/// Zeiger nach `auswerten` (uniformes ID-Handoff, ADR-0012: Tabelle = Wahrheit, Message = Pointer).
/// Opak; der Slice deutet sie nie.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(String);

impl WindowId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
