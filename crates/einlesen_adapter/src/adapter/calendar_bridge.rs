use einlesen_domain::{Calendar, CalendarDay, NonEmptyString};

use crate::abstractions::CalendarSeam;

/// Port-Bridge: mappt den öffentlichen [`CalendarSeam`] (den IO erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Calendar`, den der Handler verlangt — analog
/// `SftpPullerAdapter : IFilePuller` der C#-Referenz. Der Handler kennt **nur** den
/// Domänen-Port; das Mappen der äußeren (primitiven) Form auf den Domänentyp lebt **hier**,
/// nicht im Handler.
pub(crate) struct CalendarBridge<'a> {
    pub(crate) seam: &'a dyn CalendarSeam,
}

impl Calendar for CalendarBridge<'_> {
    fn parse(&self, raw: &NonEmptyString) -> Option<CalendarDay> {
        // Der Seam trägt die **ganze** Gültigkeit (inkl. Bereich): ein ungültiges Datum ist
        // schon `None`. Bleibt es `Some`, bündelt der (nun unfehlbare) Konstruktor die Teile.
        self.seam
            .parse(raw.as_str())
            .map(|parts| CalendarDay::from_parts(parts.year, parts.month, parts.day))
    }
}
