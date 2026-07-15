use obfuskierung_domain::{NonEmptyString, Obfuscator, Replacement, Seed};

use crate::abstractions::ObfuscatorSeam;

/// Die **Port-Bridge**: erfüllt den von der Domäne verlangten Port `obfuskierung_domain::Obfuscator`
/// und mappt ihn auf den öffentlichen [`ObfuscatorSeam`] (den io erfüllt). „außen → innen": der
/// Seam spricht Primitive (`u64`/`&str`/`String`), die Bridge übersetzt sie in die Domänen-Typen
/// (`Seed`/`NonEmptyString`/`Replacement`). Vorbild: `CalendarBridge`/`MoneyBridge` des
/// Einlese-Slice.
pub(crate) struct ObfuscatorBridge<'a> {
    pub(crate) seam: &'a dyn ObfuscatorSeam,
}

impl Obfuscator for ObfuscatorBridge<'_> {
    fn obfuscate(&self, seed: Seed, plaintext: &NonEmptyString) -> Replacement {
        Replacement::from(self.seam.obfuscate(seed.value(), plaintext.as_str()))
    }
}
