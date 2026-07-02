use domain::Rolle;

/// Ergebnis der Nachschlage-Frage „ist diese Gegen-IBAN eine eigene?".
///
/// Trägt den Eigen/Fremd- **und** den Rollen-Aspekt in **einem** Typ: „eigen
/// ohne Rolle" und „fremd mit Rolle" sind damit unkonstruierbar. Genau das
/// belegt die Eine-Quelle-Zusage — dieselbe Instanz beantwortet beide Fragen in
/// einem Zug, es gibt keine zwei getrennten Listen, die auseinanderlaufen
/// könnten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kontozuordnung {
    /// Ein eigenes Konto — mit seiner Rolle.
    Eigen(Rolle),
    /// Ein fremdes Konto.
    Fremd,
}
