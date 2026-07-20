use crate::value_objects::WindowId;

/// Das domänen-interne Kommando der **pro-Fenster**-Reichweite: „werte **dieses** eingefrorene
/// Fenster aus". Es trägt **nur die ID** — der Handler bekommt einen Zeiger, keine eingebetteten
/// Daten, und holt das Aggregat selbst über den Lese-Port (uniformes ID-Handoff, ADR-0012).
///
/// Der **Typ trägt die Absicht** (ADR-0005): die zweite, fenster-übergreifende Reichweite bekommt
/// ihr eigenes Kommando und ihren eigenen einzweckigen Handler, kein Flag an diesem hier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluateWindow {
    window: WindowId,
}

impl EvaluateWindow {
    #[must_use]
    pub fn new(window: WindowId) -> Self {
        Self { window }
    }

    pub(crate) fn window(&self) -> &WindowId {
        &self.window
    }

    /// Gibt die ID heraus — das Ergebnis hält sie als Fremdschlüssel auf sein Quell-Fenster fest.
    pub(crate) fn into_window(self) -> WindowId {
        self.window
    }
}
