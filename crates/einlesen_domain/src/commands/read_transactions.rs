use super::RawLine;

/// Das **domänen-interne Kommando** des Slice, das der
/// [`ReadHandler`](crate::ReadHandler) verarbeitet: es trägt **den ganzen Batch** —
/// alle Rohzeilen der Quelle bereits als Domänen-VOs
/// ([`RawLine`], [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md):
/// „der Command trägt, was die Domäne für ihren Task braucht — der ganze Batch, nicht
/// zeilenweise"). Der Adapter baut es aus dem Wire-DTO (ACL, „außen → innen: Command →
/// Domänen-VO"); die Domäne selbst kennt keine Wire-/serde-Form.
pub struct ReadTransactions {
    pub(crate) lines: Vec<RawLine>,
}

impl ReadTransactions {
    /// Baut das Kommando aus dem am Rand geprüften Batch (Adapter-Abbildung).
    pub fn new(lines: Vec<RawLine>) -> Self {
        Self { lines }
    }
}
