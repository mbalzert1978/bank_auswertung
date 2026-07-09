/// Das führende Vorzeichen des Betragsfelds, wie der `Money`-Port es **dekodiert**
/// liefert — bewusst **drei-wertig**: ein fehlendes Vorzeichen ist ein eigener Fall
/// (`Missing`) und wird nie auf positiv gedefaultet, sonst ginge die terminale
/// `SignMissing`-Bedeutung verloren. Das *Erkennen/Abstreifen* des Vorzeichens im rohen
/// Feld ist Format-Mechanik und lebt hinter dem IO-Seam (ADR-0008), nicht hier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Positive,
    Missing,
}

/// Die Richtung eines Umsatzes — der einzige Ort, an dem das Vorzeichen lebt (ADR-0005),
/// sodass ein negativer [`Amount`](crate::Amount) unkonstruierbar ist. Die **Bedeutung**
/// des drei-wertigen Vorzeichens lebt hier (`from_sign`);
/// `Amount::with_direction` ruft sie auf, statt sie ein zweites Mal zu führen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Incoming,
    Outgoing,
}

impl Direction {
    /// Deutet das drei-wertige Vorzeichen: `Negative` → `Outgoing`, `Positive` →
    /// `Incoming`. Ein fehlendes Vorzeichen macht die Richtung nicht deutbar (`None`) —
    /// kein stilles `Incoming`; die terminale `SignMissing`-Meldung erzeugt der Aufrufer
    /// (`Amount::with_direction`), nicht diese Abbildung.
    pub fn from_sign(sign: Sign) -> Option<Self> {
        match sign {
            Sign::Negative => Some(Self::Outgoing),
            Sign::Positive => Some(Self::Incoming),
            Sign::Missing => None,
        }
    }
}
