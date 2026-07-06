use crate::value_objects::{Kalendertag, NonEmptyString};

/// Der von der Domäne **verlangte** Port, der einen rohen Datumswert in einen
/// `Kalendertag` deutet. Die Domäne bleibt dependency-frei; die robuste Kalender-Prüfung
/// (Schaltjahr, Monatslängen) lebt hinter diesem Port in `io` (mit `chrono`), die Tests
/// unterschieben einen Fake. Der Vertrag ist wert-typisiert: die Domäne fordert einen
/// `NonEmptyString` an, **kein** primitives `&str` — der Adapter mappt den rohen
/// CSV-Wert. Der Port trägt **nur** die eine Fehlbedeutung „kein gültiges Datum" als
/// `None` (nicht das ganze Slice-Fehler-Enum); das Aggregat mappt `None` an einem Ort auf
/// `DatumUngültig`. Präzedenz: `obfuskierung_domain` verlangt so den `ZuordnungsSpeicher`
/// mit einem port-eigenen Fehler.
pub trait Kalender {
    /// Deutet den rohen Datumswert zu einem `Kalendertag`, oder `None`, wenn er kein
    /// gültiges Datum ist.
    fn deute(&self, roh: &NonEmptyString) -> Option<Kalendertag>;
}
