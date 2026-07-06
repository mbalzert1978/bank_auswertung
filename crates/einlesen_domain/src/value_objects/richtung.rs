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
    /// Deutet das **führende Vorzeichen** eines Betragsfelds und streift es ab:
    /// `-` → Ausgang, `+` → Eingang, jeweils mit den Restziffern. Ohne führendes `+`/`-`
    /// ist die Richtung nicht deutbar (`None`) — kein stilles `Eingang`. Der einzige Ort
    /// der Vorzeichen-Grammatik.
    pub fn aus_vorzeichen(feld: &str) -> Option<(Self, &str)> {
        if let Some(ziffern) = feld.strip_prefix('-') {
            Some((Self::Ausgang, ziffern))
        } else {
            feld.strip_prefix('+')
                .map(|ziffern| (Self::Eingang, ziffern))
        }
    }
}
