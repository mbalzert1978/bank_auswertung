/// Konto-Rolle im Mehrkontenmodell (CONTEXT.md): die Funktion, die ein eigenes
/// Konto hat. Bestimmt die Deutung in der Analyse — z. B. ist ein Zufluss aufs
/// Spar-Konto Sparen, nicht Konsum.
///
/// Teil des Domänenvokabulars (UBQL), daher in der Domäne beheimatet:
/// `Gegenkonto::Eigen` trägt sie, sodass „eigenes Gegenkonto ohne Rolle"
/// unkonstruierbar ist. Die Eigene-Konten-Config (Adapter) liest sie aus TOML —
/// dafür ist `Deserialize` abgeleitet. Eine Konto-Rolle ist **kein**
/// struktureller Identifikator; die Datenschutz-Garantie (kein `Serialize` auf
/// `Iban`/`Bic`) bleibt davon unberührt.
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
