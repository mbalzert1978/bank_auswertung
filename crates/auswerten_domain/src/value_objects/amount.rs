/// Ein Geldbetrag als **vorzeichenlose** Menge in **Minor Units** der Währung (ISO 4217) — das
/// Vorzeichen lebt allein in der [`Direction`], ein negativer Betrag ist dadurch unkonstruierbar
/// (ADR-0005). Die Menge ist die kleinste Einheit **dieser** Währung, nicht die feste Skala 2 (Cent).
///
/// **Ohne Währung, bewusst:** anders als `einlesen`s `Amount` trägt dieser Betrag sie nicht mit. Die
/// **eine** Währung steht einmal am Auswertungsfenster — `fenstern` hat sie **an der Konstruktion**
/// des Fensters erzwungen. Sie an jedem Betrag zu wiederholen brächte keinen Beweis dazu, sondern
/// nur einen „was, wenn sie abweichen"-Zweig, der per Konstruktion nie feuern kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Amount(u64);

impl Amount {
    #[must_use]
    pub fn from_minor_units(minor_units: u64) -> Self {
        Self(minor_units)
    }

    #[must_use]
    pub fn minor_units(self) -> u64 {
        self.0
    }

    /// Summiert zwei Beträge — die eine Rechenoperation des Bucketings. **Sättigend:** ein
    /// Überlauf ist bei echten Kontoumsätzen unerreichbar, und ein sättigendes Maximum ist immer
    /// noch besser als ein Panic im Debug-Build oder ein stiller Wrap im Release.
    #[must_use]
    pub(crate) fn plus(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Vervielfacht den Betrag — die Hochrechnung vom typischen Betrag eines wiederkehrenden Abflusses
    /// auf sein Jahr. **Sättigend** aus demselben Grund wie [`plus`](Self::plus).
    #[must_use]
    pub(crate) fn times(self, factor: u64) -> Self {
        Self(self.0.saturating_mul(factor))
    }
}

/// Die Richtung eines Umsatzes — der einzige Ort, an dem das Vorzeichen lebt (ADR-0005), sodass ein
/// negativer [`Amount`] unkonstruierbar ist. Sie entscheidet, in welche der beiden Spalten eines
/// Monats-Buckets ein Betrag fließt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Incoming,
    Outgoing,
}
