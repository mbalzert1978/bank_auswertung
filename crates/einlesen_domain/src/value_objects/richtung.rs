/// Die Richtung eines Umsatzes — der einzige Ort, an dem das Vorzeichen lebt (ADR-0005),
/// sodass ein negativer [`Betrag`](crate::Betrag) unkonstruierbar ist. Auch die
/// **Vorzeichen-Grammatik** des Betragsfelds lebt hier (`aus_vorzeichen`); `Betrag::mit_richtung`
/// ruft sie auf, statt sie ein zweites Mal zu führen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Richtung {
    Eingang,
    Ausgang,
}

impl Richtung {
    const POSITIVE: char = '+';
    const NEGATIVE: char = '-';

    /// Deutet das **führende Vorzeichen** eines Betragsfelds und streift es ab:
    /// `-` → Ausgang, `+` → Eingang, jeweils mit den Restziffern. Ohne führendes `+`/`-`
    /// ist die Richtung nicht deutbar (`None`) — kein stilles `Eingang`. Der einzige Ort
    /// der Vorzeichen-Grammatik.
    pub fn aus_vorzeichen(feld: &str) -> Option<(Self, &str)> {
        match feld.strip_prefix(Self::NEGATIVE) {
            Some(ziffern) => Some((Self::Ausgang, ziffern)),
            None => feld
                .strip_prefix(Self::POSITIVE)
                .map(|ziffern| (Self::Eingang, ziffern)),
        }
    }
}
