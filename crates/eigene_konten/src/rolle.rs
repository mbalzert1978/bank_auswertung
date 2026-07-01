/// Konto-Rolle im Mehrkontenmodell (CONTEXT.md): die Funktion, die ein eigenes
/// Konto hat. Bestimmt die Deutung in der Analyse — z. B. ist ein Zufluss aufs
/// Spar-Konto Sparen, nicht Konsum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum Rolle {
    /// Laufendes Konto.
    Giro,
    /// Haushaltskonto.
    Haushalt,
    /// Spar-Konto.
    Spar,
    /// Für anstehende Fixkosten reserviert.
    Vorlauf,
}
