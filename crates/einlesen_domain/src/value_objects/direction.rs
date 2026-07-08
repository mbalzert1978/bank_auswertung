/// Die Richtung eines Umsatzes — der einzige Ort, an dem das Vorzeichen lebt (ADR-0005),
/// sodass ein negativer [`Amount`](crate::Amount) unkonstruierbar ist. Auch die
/// **Vorzeichen-Grammatik** des Betragsfelds lebt hier (`from_sign`); `Amount::with_direction`
/// ruft sie auf, statt sie ein zweites Mal zu führen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Incoming,
    Outgoing,
}

impl Direction {
    const POSITIVE: char = '+';
    const NEGATIVE: char = '-';

    /// Deutet das **führende Vorzeichen** eines Betragsfelds und streift es ab:
    /// `-` → `Outgoing`, `+` → `Incoming`, jeweils mit den Restziffern. Ohne führendes `+`/`-`
    /// ist die Richtung nicht deutbar (`None`) — kein stilles `Incoming`. Der einzige Ort
    /// der Vorzeichen-Grammatik.
    pub fn from_sign(feld: &str) -> Option<(Self, &str)> {
        match feld.strip_prefix(Self::NEGATIVE) {
            Some(ziffern) => Some((Self::Outgoing, ziffern)),
            None => feld
                .strip_prefix(Self::POSITIVE)
                .map(|ziffern| (Self::Incoming, ziffern)),
        }
    }
}
