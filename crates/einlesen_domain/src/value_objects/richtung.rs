/// Die Richtung eines Umsatzes — der einzige Ort, an dem das Vorzeichen lebt
/// (ADR-0005), sodass ein negativer [`Betrag`](crate::Betrag) unkonstruierbar ist. Das
/// Vorzeichen des Betragsfelds wird **an einem Ort** in `Betrag::mit_richtung` gedeutet
/// (dort entstehen Betrag und Richtung gemeinsam); dieser Summentyp trägt nur das
/// Ergebnis und hat deshalb keinen eigenen Parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Richtung {
    Eingang,
    Ausgang,
}
