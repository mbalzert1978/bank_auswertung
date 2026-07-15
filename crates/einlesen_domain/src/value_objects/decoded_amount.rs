use super::Sign;

/// Ein bereits **dekodierter** Betrag, wie ihn der `Money`-Port liefert: die
/// vorzeichenlose Cent-Menge samt zwei-wertigem [`Sign`]. Ein eigenes Wert-Objekt, damit
/// die Cent-Menge die Domänen-Grenze **nie** als nacktes `u64` quert (ADR-0005: kein
/// primitives Zahl-Rohmaterial im Domänen-Inneren). Das Format-Dekodieren selbst
/// (deutscher Tausenderpunkt, Dezimalkomma, Vorzeichen) ist Mechanik hinter dem Port
/// (ADR-0008), nicht Sache dieses VO — es trägt nur das fertige Ergebnis, aus dem
/// `Amount::with_direction` unfehlbar Betrag und Richtung baut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedAmount {
    cent: u64,
    sign: Sign,
}

impl DecodedAmount {
    /// Bündelt die dekodierten Bestandteile. Der `Money`-Port (io/Fake) ruft dies nach
    /// dem Dekodieren; ein nicht dekodierbares Feld ergibt gar kein VO (`None` am Port),
    /// nie einen halben Betrag.
    #[must_use]
    pub fn new(cent: u64, sign: Sign) -> Self {
        Self { cent, sign }
    }

    #[must_use]
    pub(crate) fn cent(&self) -> u64 {
        self.cent
    }

    #[must_use]
    pub(crate) fn sign(&self) -> Sign {
        self.sign
    }
}
