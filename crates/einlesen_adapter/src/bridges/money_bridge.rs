use einlesen_domain::{DecodedAmount, Money, NonEmptyString, Sign};

use crate::abstractions::{self, MoneySeam};

/// Port-Bridge: mappt den öffentlichen [`MoneySeam`] (den IO erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Money`, den der Handler verlangt — analog
/// [`CalendarBridge`](crate::bridges::CalendarBridge). Sie mappt **rein mechanisch**
/// (Seam-Primitive → Domänen-VO, das zwei-wertige Vorzeichen 1:1); die fachliche Deutung
/// — Vorzeichen → `Direction` — lebt in der Domäne (`Amount::with_direction`), nicht hier.
pub(crate) struct MoneyBridge<'a> {
    pub(crate) seam: &'a dyn MoneySeam,
}

impl Money for MoneyBridge<'_> {
    fn decode(&self, raw: &NonEmptyString) -> Option<DecodedAmount> {
        let decoded = self.seam.decode(raw.as_str())?;
        Some(DecodedAmount::new(decoded.cent, decoded.sign.into()))
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
