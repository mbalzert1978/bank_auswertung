use crate::value_objects::RunSpan;

/// Das domänen-interne Kommando der **fenster-übergreifenden** Reichweite: „die Historie dieses
/// Laufs ist vollständig — sieh sie dir als Ganzes an".
///
/// Es trägt **nur die Lauf-Spanne**, und die ist hier kein Beiwerk, sondern der **Scope**: sie
/// grenzt diesen Lauf gegen frühere ab. `fenstern` trägt sie im `HistorieVollständig`-Signal mit,
/// weil `auswerten` sie **nicht selbst kennen kann** — der Pager läuft neu→alt, und nur `fenstern`
/// weiß, wo der Lauf begann und wann die Historie erschöpft war.
///
/// Der **Typ trägt die Absicht** (ADR-0005): dies ist ein **anderes** Kommando als das
/// [`EvaluateWindow`](crate::EvaluateWindow) der pro-Fenster-Reichweite, kein Flag an ihm. Damit ist
/// es unkonstruierbar, den fenster-übergreifenden Lauf versehentlich aus einem `FensterGeschlossen`
/// heraus anzustoßen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluateHistory {
    span: RunSpan,
}

impl EvaluateHistory {
    #[must_use]
    pub fn new(span: RunSpan) -> Self {
        Self { span }
    }

    pub(crate) fn span(self) -> RunSpan {
        self.span
    }
}
