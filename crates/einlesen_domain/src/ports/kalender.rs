use crate::fehler::ZeilenError;
use crate::value_objects::{Kalendertag, NonEmptyString};

/// Der von der Domäne **verlangte** Port, der einen rohen Datumswert in einen
/// `Kalendertag` deutet. Die Domäne bleibt dependency-frei; die robuste Kalender-Prüfung
/// (Schaltjahr, Monatslängen) lebt hinter diesem Port in `io` (mit `chrono`), die Tests
/// unterschieben einen Fake. Der Vertrag ist wert-typisiert: die Domäne fordert einen
/// `NonEmptyString` an, **kein** primitives `&str` — der Adapter mappt den rohen
/// CSV-Wert. Präzedenz: `obfuskierung_domain` verlangt so den `ZuordnungsSpeicher`.
pub trait Kalender {
    /// Deutet den rohen Datumswert. Ein unlesbares/unmögliches Datum ist terminal
    /// (`DatumUngültig`), kein Retry.
    fn deute(&self, roh: &NonEmptyString) -> Result<Kalendertag, ZeilenError>;
}
