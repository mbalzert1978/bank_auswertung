use fenstern_domain::{DayStamp, WindowAnchor, WindowError};

use crate::abstractions::{ObfuscatedWindowSeam, SourceUnavailable};

/// Die **Port-Bridge** für den Anker: erfüllt `fenstern_domain::WindowAnchor` und mappt ihn auf den
/// öffentlichen [`ObfuscatedWindowSeam`] (den io erfüllt) — dieselbe Ablage, aus der auch die Seite
/// gelesen wird, darum derselbe Seam. Sie trägt **beide** Anker-Fragen: den Initial-Anker des
/// Anstoßes (`latest`) und den der Selbst-Fortsetzung (`latest_before`). „außen → innen": der rohe
/// Tag-Stamp wird zum [`DayStamp`], die Abwesenheit eines Buchungstags bleibt ein `None` — die
/// **gültige** Antwort, die den Lauf terminieren (bzw. beim Anstoß gar nicht erst beginnen) lässt. Ein `Err` des Seams ist **terminal** → `WindowError::SourceUnavailable`
/// mit durchgereichter io-Diagnose.
pub(crate) struct AnchorBridge<'a> {
    pub(crate) seam: &'a dyn ObfuscatedWindowSeam,
}

impl WindowAnchor for AnchorBridge<'_> {
    fn latest(&self) -> Result<Option<DayStamp>, WindowError> {
        self.seam
            .latest()
            .map(|found| found.map(DayStamp::new))
            .map_err(
                |SourceUnavailable { message }| WindowError::SourceUnavailable { reason: message },
            )
    }

    fn latest_before(&self, day: DayStamp) -> Result<Option<DayStamp>, WindowError> {
        self.seam
            .latest_before(day.value())
            .map(|found| found.map(DayStamp::new))
            .map_err(
                |SourceUnavailable { message }| WindowError::SourceUnavailable { reason: message },
            )
    }
}
