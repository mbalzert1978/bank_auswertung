//! Die echte io-Erfüllung des öffentlichen [`CalendarSeam`] des `einlesen`-Adapters
//! ([Issue 0050](../../../../docs/issues/0050-money-calendar-decode-seams-real.md)): ein **Adapter**,
//! der die battle-tested [`chrono`]-Lib auf unsere Seam-Signatur andockt — rein, kein I/O. Sie tritt
//! im End-to-End-Test an die Stelle des In-Memory-Fakes.
//!
//! `chrono` deutet das deutsche `TT.MM.JJJJ` **und** trägt die **Kalendergültigkeit** (der 30.02.
//! oder ein Nicht-Schalttag-29.02. sind kein `NaiveDate`) — genau das, was der Seam-Vertrag verlangt
//! (`CalendarDay::from_parts` in der Domäne bleibt infallibel). Wir **testen `chrono` nicht**; der
//! Adapter reicht dessen Ergebnis nur auf die adapter-eigenen `CalendarDay`-Teile durch bzw. auf
//! `None`, das die Domäne terminal auf `BookingDayInvalid`/`ValueDateInvalid` mappt.

use chrono::{Datelike, NaiveDate};
use einlesen_adapter::{CalendarDay, CalendarSeam};

/// Der echte Kalender-Seam: dockt `chrono` auf [`CalendarSeam`] an. **Zustandslos** — ein Wert genügt.
pub struct GermanCalendar;

impl CalendarSeam for GermanCalendar {
    fn parse(&self, raw: &str) -> Option<CalendarDay> {
        // `chrono` besitzt Format **und** Kalendergültigkeit: `%d.%m.%Y` deutet den deutschen Tag und
        // lehnt einen nicht existierenden ab. Wir mappen nur das Ergebnis auf unsere Seam-Teile.
        let date = NaiveDate::parse_from_str(raw, "%d.%m.%Y").ok()?;
        Some(CalendarDay {
            year: u16::try_from(date.year()).ok()?,
            month: u8::try_from(date.month()).ok()?,
            day: u8::try_from(date.day()).ok()?,
        })
    }
}
