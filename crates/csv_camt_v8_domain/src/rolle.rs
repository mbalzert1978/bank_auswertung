/// Konto-Rolle im Mehrkontenmodell ([CONTEXT.md](../../../CONTEXT.md)): die
/// Funktion, die ein eigenes Konto hat. Bestimmt später die Deutung in der
/// Analyse — z. B. ist ein Zufluss aufs Spar-Konto Sparen, nicht Konsum.
///
/// **Slice-eigener** Domänen-Wert (Suffix `Entity`): die Rolle wird bewusst je
/// Slice dupliziert statt geteilt ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
/// Der Eigene-Konten-Slice liefert seine eigene `Rolle` über eine Bridge; der
/// CSV-Adapter mappt sie an der Grenze auf diese `RolleEntity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolleEntity {
    /// Laufendes Konto.
    Giro,
    /// Haushaltskonto.
    Haushalt,
    /// Spar-Konto.
    Spar,
    /// Für anstehende Fixkosten reserviert.
    Vorlauf,
}
