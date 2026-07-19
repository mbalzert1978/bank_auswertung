use crate::value_objects::DayStamp;

/// Das domänen-interne Kommando des Fenster-Schritts: es trägt **immer** einen Anker, nie ein
/// `Option` ([ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung B) — der **Typ** trägt die Absicht, der illegale Zustand „leerer Anker" ist
/// **unkonstruierbar**, nicht „geprüft". Der Anker ist die **obere** Grenze der Seite; der Pager
/// läuft neu→alt. Der Adapter baut das Kommando aus dem Wire-Zeiger; die Domäne kennt keine
/// Wire-Form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdvanceWindow {
    anchor: DayStamp,
}

impl AdvanceWindow {
    #[must_use]
    pub fn new(anchor: DayStamp) -> Self {
        Self { anchor }
    }

    pub(crate) fn anchor(&self) -> DayStamp {
        self.anchor
    }
}
