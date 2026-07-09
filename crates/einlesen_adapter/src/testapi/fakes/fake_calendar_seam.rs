use crate::abstractions::{CalendarDay, CalendarSeam};

/// Deterministischer Fake: deutet „TT.MM.JJJJ" zu seinen Bestandteilen; ungültiges Format
/// **oder** ein außerhalb des Bereichs liegender Monat/Tag ergibt `None`. Der Seam trägt die
/// **ganze** Kalender-Gültigkeit (der Domänen-Konstruktor ist unfehlbar) — hinter dem echten
/// Seam prüft später `chrono` in IO auch die feine Regel (kein 30. Februar); der Fake bildet
/// die grobe Bereichsprüfung ab, damit das End-to-End-Verhalten „ungültig → `None` →
/// `*Invalid`" real bleibt. Implementiert den öffentlichen [`CalendarSeam`] (nicht den
/// Domänen-Port) — Vorbild `FakeSftpPuller : ISftpPuller`.
pub(crate) struct FakeCalendarSeam;

impl CalendarSeam for FakeCalendarSeam {
    fn parse(&self, raw: &str) -> Option<CalendarDay> {
        let mut parts = raw.split('.');
        let day: u8 = parts.next()?.parse().ok()?;
        let month: u8 = parts.next()?.parse().ok()?;
        let year: u16 = parts.next()?.parse().ok()?;
        if (1..=12).contains(&month) && (1..=31).contains(&day) {
            Some(CalendarDay { year, month, day })
        } else {
            None
        }
    }
}
