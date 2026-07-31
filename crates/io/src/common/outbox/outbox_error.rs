use std::fmt;

/// Fehlerbild bei der Verarbeitung **einer** Outbox-Nachricht — das `E` im
/// `Result<(), OutboxError>`, das ein [`OutboxHandler`](super::OutboxHandler)
/// zurückgibt. `Ok(())` heißt „verarbeitet" (das frühere `Handled`); die
/// Fehler-Varianten steuern, was der Prozessor mit der Nachricht tut.
///
/// Löst die C#-nahen Typen `HandleOutcome` **und** `Severity` ab: Rusts `Result`
/// trägt Erfolg/Misserfolg selbst, also lebt hier nur noch das Fehlerbild. Seit dem
/// Kernel-Flip
/// ([ADR-0011](../../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md))
/// baut **`io`** diesen Fehler; der Slice gibt nur seinen **nackten Domänen-Fehler**
/// zurück und kennt `OutboxError` nicht mehr.
///
/// **Kein Schweregrad, keine Handlungsanweisung.** Die Varianten benennen, **wo** die
/// Verarbeitung brach — nicht, wie schlimm es ist oder was danach zu geschehen hätte.
/// Die frühere Erzählung, `io` klassifiziere die Disposition in Warning/Retry/Critical,
/// war ein Missverständnis, das
/// [ADR-0012](../../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
/// ausgeräumt hat: die Pipeline ist **durchweg terminal**. Wie ein terminaler Fehlschlag
/// zu behandeln wäre — Versuchszählung, Dead-Letter — ist dort ausdrücklich **vertagt**
/// und heute **nicht gebaut**
/// ([Issue 0085](../../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md));
/// eine gescheiterte Nachricht bleibt schlicht offen liegen.
///
/// Geschlossener Summentyp — nur diese Fälle sind konstruierbar. Der Kernel bleibt
/// dependency-frei: `Display`/`Error` sind von Hand geschrieben, kein `thiserror`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxError {
    /// Die **Arbeitseinheit** brach — das Commit der Transaktion, nicht die Fachlichkeit.
    /// Die Nachricht bleibt offen; ein späterer Lauf nimmt sie wieder auf (at-least-once).
    Retry {
        /// Die Ursache des gescheiterten Commits.
        reason: String,
    },
    /// Der **Handler** brach: der Slice gab seinen nackten, redigierten Domänen-Fehler
    /// zurück. `reason` ist genau dieser Fehler, unverändert durchgereicht.
    Warning {
        /// Der redigierte Domänen-Fehler des Slice.
        reason: String,
    },
    /// Die **Hülle** trug nicht: leerer oder ungültiger Inhalt, kein Consumer für den
    /// Dispatch-Key — der Fehlschlag liegt vor der Fachlichkeit.
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
            // Jede Variante benennt den **Ort** des Bruchs, ohne ihn zu bewerten und ohne das
            // „gescheitert" der Hülle zu wiederholen, die sie einbettet
            // ([`RunError::Message`](crate::run::RunError)).
            OutboxError::Retry { reason } => write!(f, "beim Commit: {reason}"),
            OutboxError::Warning { reason } => write!(f, "im Slice: {reason}"),
            OutboxError::Critical { message, source } => {
                write!(f, "in der Nachrichten-Hülle aus {source}: {message}")
            }
        }
    }
}

impl std::error::Error for OutboxError {}
