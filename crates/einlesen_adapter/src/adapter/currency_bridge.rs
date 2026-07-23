use einlesen_domain::{Currencies, Currency, NonEmptyString};

use crate::abstractions::CurrencySeam;

/// Port-Bridge: mappt den öffentlichen [`CurrencySeam`] (den IO erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Currencies`, den der Handler verlangt — analog
/// [`CalendarBridge`](crate::adapter::CalendarBridge). Der Handler kennt **nur** den Domänen-Port;
/// das Mappen der äußeren (primitiven) Form auf den Domänentyp [`Currency`] — Code **und** Minor
/// Unit — lebt **hier**, nicht im Handler. Ein von der Lib nicht geführter Code ist schon `None`.
pub(crate) struct CurrencyBridge<'a> {
    pub(crate) seam: &'a dyn CurrencySeam,
}

impl Currencies for CurrencyBridge<'_> {
    fn resolve(&self, raw: &NonEmptyString) -> Option<Currency> {
        self.seam
            .resolve(raw.as_str())
            .map(|resolved| Currency::from_parts(resolved.code, resolved.minor_unit))
    }
}
