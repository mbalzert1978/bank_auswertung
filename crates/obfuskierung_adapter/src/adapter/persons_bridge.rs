use obfuskierung_domain::{KnownPersons, KnownPersonsSource, NonEmptyString, ObfuscationError};

use crate::abstractions::{KnownPersonsSeam, PersonsUnavailable};

/// Die **Port-Bridge** für die Namensliste: erfüllt `obfuskierung_domain::KnownPersonsSource` und
/// mappt ihn auf den öffentlichen [`KnownPersonsSeam`] (den io erfüllt). „außen → innen": die rohen
/// Namen werden zu [`NonEmptyString`] (leere Einträge fallen weg) und zur Domänen-Liste
/// [`KnownPersons`]; ein [`PersonsUnavailable`] wird zum redigierten terminalen
/// [`ObfuscationError::PersonsUnavailable`] mit der durchgereichten io-Diagnose.
pub(crate) struct PersonsBridge<'a> {
    pub(crate) seam: &'a dyn KnownPersonsSeam,
}

impl KnownPersonsSource for PersonsBridge<'_> {
    fn known_persons(&self) -> Result<KnownPersons, ObfuscationError> {
        self.seam
            .known_persons()
            .map(|names| KnownPersons::new(names.iter().filter_map(NonEmptyString::new)))
            .map_err(
                |PersonsUnavailable { message }| ObfuscationError::PersonsUnavailable {
                    reason: message,
                },
            )
    }
}
