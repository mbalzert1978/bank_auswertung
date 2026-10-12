use crate::value_objects::{DayStamp, RunId};

/// Das domänen-interne Kommando des Fenster-Schritts: es trägt **immer** einen Anker, nie ein
/// `Option` ([ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung B) — der **Typ** trägt die Absicht, der illegale Zustand „leerer Anker" ist
/// **unkonstruierbar**, nicht „geprüft". Der Anker ist die **obere** Grenze der Seite; der Pager
/// läuft neu→alt. Der Adapter baut das Kommando aus dem Wire-Zeiger; die Domäne kennt keine
/// Wire-Form.
///
/// Daneben trägt es den **Lauf-Gipfel** (`run_top`): den Anker, mit dem der Lauf begann. Er reist
/// unverändert durch die ganze Selbst-Fortsetzung mit und ist die **Obergrenze der Lauf-Spanne**,
/// die der Schritt beim Versiegen der Historie als `HistoryComplete` publiziert. Ohne ihn wäre die
/// Lauf-Spanne aus einem einzelnen Kommando nicht rekonstruierbar — der Handler ist zustandslos und
/// kennt sonst nur seine eigene Seite.
///
/// Ebenso unverändert reist die **Lauf-Identität** mit (ADR-0016): das Fenster dieser Seite gehört
/// zu genau diesem Lauf, und der Abschluss nennt ihn, damit `auswerten` nur dessen Fenster liest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceWindow {
    anchor: DayStamp,
    run_top: DayStamp,
    run: RunId,
}

impl AdvanceWindow {
    #[must_use]
    pub fn new(anchor: DayStamp, run_top: DayStamp, run: RunId) -> Self {
        Self {
            anchor,
            run_top,
            run,
        }
    }

    /// Der Lauf, zu dem diese Seite gehört.
    #[must_use]
    pub fn run(&self) -> &RunId {
        &self.run
    }

    /// Die **obere** Grenze der Seite.
    #[must_use]
    pub fn anchor(&self) -> DayStamp {
        self.anchor
    }

    /// Der Anker, mit dem der Lauf begann — die Obergrenze der Lauf-Spanne.
    #[must_use]
    pub fn run_top(&self) -> DayStamp {
        self.run_top
    }
}
