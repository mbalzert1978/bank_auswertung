use einlesen_domain::{Currency, DecodedAmount, Money, NonEmptyString, Sign};

use crate::abstractions::{self, MoneySeam};

/// Port-Bridge: mappt den öffentlichen [`MoneySeam`] (den IO erfüllt) auf den internen Domänen-Port
/// `einlesen_domain::Money`, den der Handler verlangt — analog
/// [`CalendarBridge`](crate::adapter::CalendarBridge). Sie **skaliert währungs-bewusst**: der Seam
/// liefert die unskalierten Bestandteile (Ziffernmenge + Nachkommastellen), die bereits gedeutete
/// [`Currency`] trägt die **Minor Unit** (der `CurrencySeam` hat sie geliefert), und die Bridge bringt
/// den Betrag auf die Minor Units **dieser** Währung. Das ist reine Mechanik (kein Rohwert, kein
/// Domänen-Entscheid); die fachliche Deutung — Vorzeichen → `Direction` — lebt weiter in der Domäne
/// (`Amount::with_direction`), nicht hier.
pub(crate) struct MoneyBridge<'a> {
    pub(crate) seam: &'a dyn MoneySeam,
}

impl Money for MoneyBridge<'_> {
    fn decode(&self, raw: &NonEmptyString, currency: &Currency) -> Option<DecodedAmount> {
        let decoded = self.seam.decode(raw.as_str())?;
        let minor_units = rescale(decoded.units, decoded.decimal_places, currency.minor_unit())?;
        Some(DecodedAmount::new(minor_units, decoded.sign.into()))
    }
}

/// Skaliert die unskalierte Menge (`units / 10^decimal_places`) auf die Minor Units einer Währung
/// mit `exponent` Nachkommastellen — reine, überlauf-geprüfte Ganzzahl-Mechanik. Hat das Rohfeld
/// **weniger** Stellen als die Minor Unit, wird mit `10^(exponent − decimal_places)`
/// hochskaliert; hat es **mehr**, muss die Menge glatt durch `10^(decimal_places − exponent)`
/// teilbar sein, sonst ist der Betrag für diese Währung zu genau (z. B. `1234,56 JPY`) → `None`,
/// was die Domäne terminal auf `AmountInvalid` mappt.
fn rescale(units: u64, decimal_places: u32, exponent: u32) -> Option<u64> {
    if exponent >= decimal_places {
        let factor = 10u64.checked_pow(exponent - decimal_places)?;
        units.checked_mul(factor)
    } else {
        let divisor = 10u64.checked_pow(decimal_places - exponent)?;
        units.is_multiple_of(divisor).then_some(units / divisor)
    }
}

/// Übersetzt das mechanische Seam-Vorzeichen 1:1 auf das Domänen-`Sign` — infallibel, ein-eingabig.
/// Der Zieltyp ist domänen-fremd, die Quelle adapter-lokal, darum ist der `From`-Impl erlaubt.
impl From<abstractions::Sign> for Sign {
    fn from(sign: abstractions::Sign) -> Self {
        match sign {
            abstractions::Sign::Negative => Sign::Negative,
            abstractions::Sign::Positive => Sign::Positive,
        }
    }
}
