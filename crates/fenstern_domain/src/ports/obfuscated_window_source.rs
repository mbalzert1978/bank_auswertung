use crate::entities::WindowedBooking;
use crate::error::WindowError;
use crate::value_objects::WindowSpan;

/// Der von der Domäne **verlangte** Lese-Port, über den der Handler die obfuskierten Buchungen
/// **einer Seite** holt (io: `WHERE booking_day BETWEEN from AND to`). **io erfüllt ihn**; in den
/// Specs trägt ihn ein Fake.
///
/// **Fehlbar und terminal** ([`WindowError`]): ein Fehlschlag der Beschaffung ist ein Fehler, keine
/// Abwesenheit. `Ok(vec![])` ist dagegen eine **gültige Antwort** — dass daraus kein Fenster wird,
/// entscheidet die Konstruktion des Aggregats, nicht der Port.
pub trait ObfuscatedWindowSource {
    /// Liest die obfuskierten Buchungen der Spanne (in Lesereihenfolge), oder scheitert terminal.
    fn read(&self, span: &WindowSpan) -> Result<Vec<WindowedBooking>, WindowError>;
}
