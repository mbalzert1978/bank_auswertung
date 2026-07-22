use super::{DecodedAmount, Direction};

/// Die Währung eines Betrags — ein Wert-Objekt aus **ISO-4217-Code** und der **Minor Unit**
/// (Nachkommastellen) dieser Währung. Die Domäne bleibt **Wahrheit für die Währung**, ohne eine
/// handgepflegte Tabelle zu führen: **welche** Codes gültig sind und **wie viele** Nachkommastellen
/// jeder hat, deutet der von der Domäne verlangte [`Currencies`](crate::Currencies)-Port (in `io`
/// mit einer battle-tested ISO-4217-Lib), genau wie [`Calendar`](crate::Calendar) das Datum deutet.
/// Ein unbekannter Code hat den Port schon als `None` verlassen (das Aggregat mappt das terminal auf
/// `CurrencyUnknown`); dieses VO bündelt danach nur die geprüften Teile. Die Minor Unit ist die
/// Skala, auf die der [`Money`](crate::Money)-Port den Betrag bringt (EUR = 2, JPY = 0, BHD = 3),
/// **nicht** eine feste Skala 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Currency {
    code: String,
    minor_unit: u32,
}

impl Currency {
    /// Bündelt die Teile einer **bereits gedeuteten** Währung — der Konstruktor, den der
    /// [`Currencies`](crate::Currencies)-Port (in `io`) nach erfolgreichem Auflösen aufruft.
    /// **Unfehlbar:** die Gültigkeit (kennt die ISO-4217-Lib den Code?) trägt der Port, ein
    /// unbekannter Code ist dort schon `None` geworden; hier gibt es nichts mehr abzuweisen.
    #[must_use]
    pub fn from_parts(code: String, minor_unit: u32) -> Self {
        Self { code, minor_unit }
    }

    /// Der kanonische ISO-4217-Code — die Identität der Währung, die die Projektion mitträgt.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Die Nachkommastellen der Minor Unit dieser Währung — die Skala, auf die der `Money`-Port den
    /// Betrag bringt.
    #[must_use]
    pub fn minor_unit(&self) -> u32 {
        self.minor_unit
    }
}

/// Ein Geldbetrag als **vorzeichenlose** Menge in **Minor Units** der Währung (ISO 4217) — das
/// Vorzeichen lebt allein in der [`Direction`](crate::Direction), ein negativer Betrag ist dadurch
/// unkonstruierbar (ADR-0005). Die Menge ist die kleinste Einheit **dieser** Währung (EUR-Cent =
/// Skala 2, JPY = Skala 0, BHD = Skala 3), **nicht** die feste Skala 2. Betrag und Richtung entstehen
/// **gemeinsam** aus dem vom `Money`-Port dekodierten [`DecodedAmount`](crate::DecodedAmount); das
/// *Zahlenformat*-Dekodieren und die **währungs-bewusste Skalierung** auf die Minor Units leben hinter
/// dem `Money`-Port im Adapter/io (ADR-0008), nicht in der Domäne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    minor_units: u64,
    currency: Currency,
}

impl Amount {
    /// Baut Betrag und Richtung **gemeinsam** aus dem vom `Money`-Port dekodierten
    /// [`DecodedAmount`](crate::DecodedAmount): die vorzeichenlose Minor-Unit-Menge wird zum Betrag,
    /// das zwei-wertige Vorzeichen deutet `Direction::from_sign` (dort lebt seine Bedeutung).
    /// **Unfehlbar** — ein nicht dekodierbares oder für die Währung zu genaues Feld hat den Port schon
    /// als `None` verlassen (im Aggregat `AmountInvalid`); hier gibt es nichts mehr abzuweisen. Die
    /// gedeutete Währung reist mit dem Betrag mit. So lebt das Vorzeichen allein in der Richtung.
    #[must_use]
    pub(crate) fn with_direction(decoded: DecodedAmount, currency: Currency) -> (Self, Direction) {
        let direction = Direction::from_sign(decoded.sign());
        (
            Self {
                minor_units: decoded.minor_units(),
                currency,
            },
            direction,
        )
    }

    #[must_use]
    pub fn minor_units(&self) -> u64 {
        self.minor_units
    }

    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }
}
