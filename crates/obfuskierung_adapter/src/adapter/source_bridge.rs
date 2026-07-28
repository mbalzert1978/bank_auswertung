use obfuskierung_domain::{BookingId, IngestedBooking, RawBookingSource, SourceError};

use crate::abstractions::{IngestedSourceSeam, SourceUnavailable};

/// Die **Port-Bridge** für den Lese-Port: erfüllt `obfuskierung_domain::RawBookingSource` und mappt
/// ihn auf den öffentlichen [`IngestedSourceSeam`] (den io erfüllt). „außen → innen": der Seam liefert
/// die rohe [`IngestedRow`](crate::models::IngestedRow), die Bridge deutet sie in das Domänen-Aggregat.
///
/// Sie schließt die **drei** Ausgänge des Seams zu den drei terminalen Domänen-Fehlern: ein `Err` →
/// [`SourceError::SourceUnavailable`] (io-Diagnose durchgereicht), ein `Ok(None)` →
/// [`SourceError::NotFound`] mit der **opaken ID**, eine Zeile mit fehlendem Pflichtfeld →
/// [`SourceError::MissingMandatoryField`] (redigierter Feldname).
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn IngestedSourceSeam,
}

impl RawBookingSource for SourceBridge<'_> {
    fn read(&self, id: &BookingId) -> Result<IngestedBooking, SourceError> {
        self.seam
            .read(id.as_str())
            .map_err(
                |SourceUnavailable { message }| SourceError::SourceUnavailable { reason: message },
            )?
            .ok_or_else(|| SourceError::NotFound(id.clone()))?
            .to_booking(id)
            .map_err(|missing| SourceError::MissingMandatoryField {
                field: missing.field(),
            })
    }
}
