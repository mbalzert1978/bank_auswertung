use fenstern_domain::{ObfuscatedWindowSource, WindowError, WindowSpan, WindowedBooking};

use crate::abstractions::{ObfuscatedWindowSeam, SourceUnavailable};

/// Die **Port-Bridge** für den Lese-Port: erfüllt `fenstern_domain::ObfuscatedWindowSource` und
/// mappt ihn auf den öffentlichen [`ObfuscatedWindowSeam`] (den io erfüllt). „außen → innen": die
/// Domänen-[`WindowSpan`] wird zu den zwei rohen Tag-Stamps des Seams, die gelieferten Zeilen zu
/// Domänen-Sichten — inklusive der **Präsenz-Prüfung** (ACL), deren fehlender Feldname redigiert in
/// den terminalen `WindowError::MissingMandatoryField` wandert. Ein `Err` des Seams ist **terminal**
/// → `WindowError::SourceUnavailable` mit durchgereichter io-Diagnose.
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn ObfuscatedWindowSeam,
}

impl ObfuscatedWindowSource for SourceBridge<'_> {
    fn read(&self, span: &WindowSpan) -> Result<Vec<WindowedBooking>, WindowError> {
        self.seam
            .read(span.from().value(), span.to().value())
            .map_err(
                |SourceUnavailable { message }| WindowError::SourceUnavailable { reason: message },
            )?
            .iter()
            .map(|row| {
                row.to_booking()
                    .map_err(|missing| WindowError::MissingMandatoryField {
                        field: missing.field(),
                    })
            })
            .collect()
    }
}
