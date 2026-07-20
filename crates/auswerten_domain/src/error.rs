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
    /// Die **Eigene-Konten-Config** führt eine Rolle, die es nicht gibt. Terminal statt still
    /// übersprungen: ein eigenes Konto klammheimlich als *fremd* zu führen, ließe die Umbuchung
    /// darauf wie Konsum aussehen, ohne dass irgendetwas fehlschlüge.
    ///
    /// Trägt **nichts** — eine verrutschte Config-Zelle kann beliebigen Inhalt führen, und ihn
    /// zurückzuspiegeln wäre genau das Leck, das dieser Slice ausschließt.
    UnknownAccountRole,
    /// Die **Eigene-Konten-Config** stammt aus einer **anderen** Tokenisierung als das Fenster —
    /// ihre Seed-Marker stimmen nicht überein.
    ///
    /// Terminal, weil das Gegenteil der **stille** Fehler wäre: kein Token matchte, jede Umbuchung
    /// liefe lautlos als „fremd" durch, und der Bericht wiese sie als Konsum aus, ohne dass
    /// irgendetwas fehlschlüge. Lieber gar kein Ergebnis als ein falsches, das sich richtig liest.
    ///
    /// Trägt **keinen** der beiden Marker — sie benennt die Lage, nicht die Werte.
    SeedMismatch,
    /// Das **Kategorie-Register** ließ sich nicht befragen. Terminal, weil das Gegenteil still
    /// falsch wäre: hielte der Lauf bei ausgefallener Auskunft einfach alles für unbekannt, ginge
    /// jede bereits gelernte Korrektur erneut ans Orakel — und dessen Antwort überschriebe womöglich
    /// genau die Korrektur, die sie hervorgebracht hat.
    ///
    /// Trägt die io-Diagnose (Zugriffs-Mechanik, nie Zeilen-/Feldinhalt).
    RegistryUnavailable { reason: String },
    /// Das **Orakel** war nicht erreichbar oder hat nicht verwertbar geantwortet. Trägt die
    /// io-Diagnose der Zugriffs-Mechanik — **nie** den angefragten Text und nie die Antwort selbst.
    CategorizerUnavailable { reason: String },
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
            EvaluationError::UnknownAccountRole => {
                write!(f, "unknown account role in the own-accounts configuration")
            }
            EvaluationError::SeedMismatch => write!(
                f,
                "own-accounts configuration and analysis window were tokenised under different seeds"
            ),
            EvaluationError::RegistryUnavailable { reason } => {
                write!(f, "could not query the category register: {reason}")
            }
            EvaluationError::CategorizerUnavailable { reason } => {
                write!(f, "could not ask the categoriser: {reason}")
            }
        }
    }
}

impl std::error::Error for EvaluationError {}
