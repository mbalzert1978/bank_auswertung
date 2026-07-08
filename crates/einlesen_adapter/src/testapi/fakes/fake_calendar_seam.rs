use crate::abstractions::{CalendarDay, CalendarSeam};

/// Deterministischer Fake: deutet „TT.MM.JJJJ" zu seinen Bestandteilen; ungültiges Format ergibt
/// `None`. Implementiert den öffentlichen [`CalendarSeam`] (nicht den Domänen-Port) — hinter dem
/// echten Seam steckt später `chrono` in IO (Vorbild `FakeSftpPuller : ISftpPuller`).
pub(crate) struct FakeCalendarSeam;

impl CalendarSeam for FakeCalendarSeam {
    fn parse(&self, raw: &str) -> Option<CalendarDay> {
        let mut parts = raw.split('.');
        let day: u8 = parts.next()?.parse().ok()?;
        let month: u8 = parts.next()?.parse().ok()?;
        let year: u16 = parts.next()?.parse().ok()?;
        Some(CalendarDay { year, month, day })
    }
}
