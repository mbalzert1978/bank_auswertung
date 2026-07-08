use einlesen_domain::{Calendar, CalendarDay, NonEmptyString};

use crate::abstractions::CalendarSeam;

/// Port-Bridge: mappt den öffentlichen [`CalendarSeam`] (den IO erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Calendar`, den der Handler verlangt — analog
/// `SftpPullerAdapter : IFilePuller` der C#-Referenz. Der Handler kennt **nur** den
/// Domänen-Port; das Mappen der äußeren (primitiven) Form auf den Domänentyp lebt **hier**,
/// nicht im Handler.
pub(crate) struct CalendarBridge<'a, S: CalendarSeam> {
    pub(crate) seam: &'a S,
}

impl<S: CalendarSeam> Calendar for CalendarBridge<'_, S> {
    fn parse(&self, raw: &NonEmptyString) -> Option<CalendarDay> {
        let parts = self.seam.parse(raw.as_str())?;
        CalendarDay::from_parts(parts.year, parts.month, parts.day)
    }
}
