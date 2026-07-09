use einlesen_domain::{Money, NonEmptyString, Sign};

use crate::abstractions::{self, MoneySeam};

/// Port-Bridge: mappt den öffentlichen [`MoneySeam`] (den IO erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Money`, den der Handler verlangt — analog
/// [`CalendarBridge`](crate::bridges::CalendarBridge). Sie mappt **rein mechanisch**
/// (Seam-Primitive → Domänen-Werte, das drei-wertige Vorzeichen 1:1); die fachliche
/// Deutung — Vorzeichen → `Direction` bzw. fehlend → `SignMissing` — lebt in der Domäne
/// (`Amount::with_direction`), nicht hier.
pub(crate) struct MoneyBridge<'a, S: MoneySeam> {
    pub(crate) seam: &'a S,
}

impl<S: MoneySeam> Money for MoneyBridge<'_, S> {
    fn decode(&self, raw: &NonEmptyString) -> Option<(u64, Sign)> {
        let decoded = self.seam.decode(raw.as_str())?;
        let sign = match decoded.sign {
            abstractions::Sign::Negative => Sign::Negative,
            abstractions::Sign::Positive => Sign::Positive,
            abstractions::Sign::Missing => Sign::Missing,
        };
        Some((decoded.cent, sign))
    }
}
