use std::fmt;

use crate::value_objects::WindowId;

/// Warum ein Schritt des Slice terminal scheitert — **die Pipeline ist durchweg terminal**
/// (ADR-0012): **kein** Dead-Letter, **kein** Retry im Slice. Lesen und Ablegen sind **je fehlbar**,
/// und jede ihrer Fehlvarianten endet hier.
///
/// **Redaktion per Konstruktion:** keine Variante trägt je einen Rohwert. Die Abwesenheit des
/// Aggregats trägt **nur die opake [`WindowId`]**, die Präsenz-Verletzung **nur den statischen
/// Feldnamen**, und die Mechanik-Fehlschläge die **io-Diagnose** (Zugriffs-Mechanik), zu deren
/// Inhaltsfreiheit sich der jeweilige Seam vertraglich verpflichtet.
///
/// Die Domäne ist dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein `thiserror`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    /// Zur ID liegt kein Auswertungsfenster in der Ablage. Trägt die **opake** [`WindowId`] (kein
    /// Rohwert) — der Zeiger zeigt ins Leere, und das ist terminal, keine Abwesenheit zum
    /// Weiterrechnen.
    WindowNotFound(WindowId),
    /// Das gelesene Fenster hat ein leeres/fehlendes Pflichtfeld — trägt **nur** den statischen
    /// Feldnamen, nie den Rohwert.
    MissingMandatoryField { field: &'static str },
    /// Das Auswertungsfenster ließ sich nicht beschaffen. Trägt die io-Diagnose (Beschaffungs-
    /// Mechanik, nie Satz-/Feldinhalt).
    SourceUnavailable { reason: String },
    /// Die Ablage des Pro-Fenster-Ergebnisses ist gescheitert. Trägt die io-Diagnose
    /// (Zugriffs-Mechanik).
    StorageUnavailable { reason: String },
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationError::WindowNotFound(id) => {
                write!(f, "referenced analysis window not found: {}", id.as_str())
            }
            EvaluationError::MissingMandatoryField { field } => {
                write!(f, "mandatory field missing in analysis window: {field}")
            }
            EvaluationError::SourceUnavailable { reason } => {
                write!(f, "could not read the analysis window: {reason}")
            }
            EvaluationError::StorageUnavailable { reason } => {
                write!(f, "could not store the window evaluation: {reason}")
            }
        }
    }
}

impl std::error::Error for EvaluationError {}
