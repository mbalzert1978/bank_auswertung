use crate::error::ObfuscationError;
use crate::value_objects::KnownPersons;

/// Der von der Domäne **verlangte** Port, über den der Handler die **bekannten Personen-Namen**
/// bekommt, die der Freitext-Engpass ersetzt ([`KnownPersons`]). **io erfüllt ihn** (heute: die
/// Empfänger-Namen aus dem einlesen-Bestand plus eine lokale Namensliste); in den Specs trägt ihn ein
/// Fake. Die Domäne liest selbst weder Datei noch Datenbank.
///
/// Der Handler fragt ihn **je Buchung** — so kennt er auch die Namen, die erst im laufenden Import
/// dazugekommen sind.
///
/// **Fehlbar und terminal** ([`ObfuscationError::PersonsUnavailable`]): ohne Namensliste wird nicht
/// obfuskiert, statt mit einer leeren Liste still zu wenig zu scrubben.
pub trait KnownPersonsSource {
    /// Die bekannten Personen-Namen, oder ein terminaler [`ObfuscationError`].
    fn known_persons(&self) -> Result<KnownPersons, ObfuscationError>;
}
