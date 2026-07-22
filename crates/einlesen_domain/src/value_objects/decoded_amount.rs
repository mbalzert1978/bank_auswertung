use super::Sign;

/// Ein bereits **dekodierter und währungs-bewusst skalierter** Betrag, wie ihn der
/// `Money`-Port liefert: die vorzeichenlose Menge in **Minor Units** der Währung (ISO 4217)
/// samt zwei-wertigem [`Sign`]. Ein eigenes Wert-Objekt, damit die Menge die Domänen-Grenze
/// **nie** als nacktes `u64` quert (ADR-0005: kein primitives Zahl-Rohmaterial im
/// Domänen-Inneren). Das Format-Dekodieren und die Skalierung auf die Minor Units **dieser**
/// Währung sind Mechanik hinter dem Port (ADR-0008), nicht Sache dieses VO — es trägt nur das
/// fertige Ergebnis, aus dem `Amount::with_direction` unfehlbar Betrag und Richtung baut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedAmount {
    minor_units: u64,
    sign: Sign,
}

impl DecodedAmount {
    /// Bündelt die dekodierten Bestandteile. Der `Money`-Port (io/Fake) ruft dies nach dem
    /// Dekodieren **und** Skalieren auf die Minor Units der Währung; ein nicht dekodierbares
    /// (oder für die Währung zu genaues) Feld ergibt gar kein VO (`None` am Port), nie einen
    /// halben Betrag.
    #[must_use]
    pub fn new(minor_units: u64, sign: Sign) -> Self {
        Self { minor_units, sign }
    }

    #[must_use]
    pub(crate) fn minor_units(&self) -> u64 {
        self.minor_units
    }

    #[must_use]
    pub(crate) fn sign(&self) -> Sign {
        self.sign
    }
}
