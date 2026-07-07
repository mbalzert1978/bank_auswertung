use einlesen_domain::{Kalender, NonEmptyString};

use crate::abstractions::KalenderSeam;

/// Port-Bridge: mappt den öffentlichen [`KalenderSeam`] (den IO erfüllt) auf den internen
/// Domänen-Port `einlesen_domain::Kalender`, den der Handler verlangt — analog
/// `SftpPullerAdapter : IFilePuller` der C#-Referenz. Der Handler kennt **nur** den
/// Domänen-Port; das Mappen der äußeren (primitiven) Form auf den Domänentyp lebt **hier**,
/// nicht im Handler.
pub(crate) struct KalenderBridge<'a, S: KalenderSeam> {
    pub(crate) seam: &'a S,
}

impl<S: KalenderSeam> Kalender for KalenderBridge<'_, S> {
    fn deute(&self, roh: &NonEmptyString) -> Option<einlesen_domain::Kalendertag> {
        let teile = self.seam.deute(roh.as_str())?;
        einlesen_domain::Kalendertag::aus_bestandteilen(teile.jahr, teile.monat, teile.tag)
    }
}
