/// Konto-Rolle im Mehrkontenmodell ([CONTEXT.md](../../../CONTEXT.md)): die
/// Funktion, die ein eigenes Konto hat. Bestimmt später die Deutung in der
/// Analyse — z. B. ist ein Zufluss aufs Spar-Konto Sparen, nicht Konsum.
///
/// Domänen-Wert dieses Slice (Suffix `Entity` nach der Namenskonvention aus
/// [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)). Das
/// UBQL-DTO am Rand heißt schlicht `Rolle`; die Übersetzung Text ↔ `RolleEntity`
/// besorgt der Adapter. Eine Konto-Rolle ist **kein** struktureller
/// Identifikator.
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
