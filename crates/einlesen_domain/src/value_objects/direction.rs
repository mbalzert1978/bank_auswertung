/// Das führende Vorzeichen des Betragsfelds, wie der `Money`-Port es **dekodiert**
/// liefert — **zwei-wertig**: die Domäne validiert nicht, sie sieht nur ein fertig
/// dekodiertes Vorzeichen. Ein Betragsfeld ohne deutbares Vorzeichen ist für sie schon
/// gar kein Betrag; der Port weist es als `None` ab (im Aggregat `AmountInvalid`). Das
/// *Erkennen/Abstreifen* des Vorzeichens im rohen Feld ist Format-Mechanik und lebt
/// hinter dem IO-Seam (ADR-0008), nicht hier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Positive,
}

/// Die Richtung eines Umsatzes — der einzige Ort, an dem das Vorzeichen lebt (ADR-0005),
/// sodass ein negativer `Amount` unkonstruierbar ist. Die **Bedeutung**
/// des Vorzeichens lebt hier (`from_sign`, total); `Amount::with_direction` ruft sie auf,
/// statt sie ein zweites Mal zu führen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Incoming,
    Outgoing,
}

impl Direction {
    /// Deutet das Vorzeichen **total**: `Negative` → `Outgoing`, `Positive` → `Incoming`.
    /// Beide Fälle sind deutbar — ein fehlendes Vorzeichen gibt es hier nicht mehr (der
    /// `Money`-Port hätte ein solches Feld schon als `None` abgewiesen).
    #[must_use]
    pub(crate) fn from_sign(sign: Sign) -> Self {
        match sign {
            Sign::Negative => Self::Outgoing,
            Sign::Positive => Self::Incoming,
        }
    }
}
