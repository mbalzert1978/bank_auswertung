use std::fmt;

/// Fehlerbild bei der Verarbeitung **einer** Outbox-Nachricht — das `E` im
/// `Result<(), OutboxError>`, das ein [`OutboxHandler`](crate::OutboxHandler)
/// zurückgibt. `Ok(())` heißt „verarbeitet" (das frühere `Handled`); die
/// Fehler-Varianten steuern, was der Prozessor mit der Nachricht tut.
///
/// Löst die C#-nahen Typen `HandleOutcome` **und** `Severity` ab: Rusts `Result`
/// trägt Erfolg/Misserfolg selbst, also lebt hier nur noch das Fehlerbild, und der
/// frühere Schweregrad ist zu den Fehler-Varianten geworden. Seit dem Kernel-Flip
/// ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md))
/// baut **`io`** diesen Fehler — die generische `OutboxConsumer` klassifiziert die
/// Disposition (Handler-Fehler → `Warning` terminal, Commit-Fehler → `Retry`,
/// leerer/ungültiger Inhalt → `Critical`); der Slice gibt nur seinen **nackten
/// Domänen-Fehler** zurück und kennt `OutboxError` nicht mehr.
///
/// Geschlossener Summentyp — nur diese Fälle sind konstruierbar. Der Kernel bleibt
/// dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein `thiserror`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxError {
    /// Transient gescheitert — die Nachricht bleibt liegen und wird in einem
    /// späteren Zyklus erneut zugestellt. `reason` ist die transiente Ursache.
    Retry {
        /// Die transiente Ursache (nach erschöpften Versuchen als Dead-Letter-Grund persistiert).
        reason: String,
    },
    /// Dauerhaft gescheitert, Schweregrad Warnung — Dead-Letter mit
    /// menschenlesbarem Grund.
    Warning {
        /// Der Grund der Warnung.
        reason: String,
    },
    /// Dauerhaft gescheitert, kritisch — Dead-Letter mit Meldung und auslösender
    /// Quelle (z. B. dem Dispatch-Key der Nachricht).
    Critical {
        /// Menschenlesbare Fehlermeldung.
        message: String,
        /// Die auslösende Quelle (z. B. der Dispatch-Key der Nachricht).
        source: String,
    },
}

impl fmt::Display for OutboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutboxError::Retry { reason } => {
                write!(f, "transient gescheitert, erneut zuzustellen: {reason}")
            }
            OutboxError::Warning { reason } => {
                write!(f, "dauerhaft gescheitert (Warnung): {reason}")
            }
            OutboxError::Critical { message, source } => {
                write!(f, "kritisch gescheitert aus {source}: {message}")
            }
        }
    }
}

impl std::error::Error for OutboxError {}
