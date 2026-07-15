use obfuskierung_domain::{BookingId, IngestedBooking, RawBookingSource, SourceError};

use crate::abstractions::IngestedSourceSeam;

/// Die **Port-Bridge** für den Lese-Port: erfüllt `obfuskierung_domain::RawBookingSource` und mappt
/// ihn auf den öffentlichen [`IngestedSourceSeam`] (den io erfüllt). „außen → innen": der Seam liefert
/// die rohe [`IngestedRow`](crate::dto::IngestedRow), die Bridge deutet sie in das Domänen-Aggregat.
/// Keine Zeile → terminaler [`SourceError::NotFound`] (opake ID); eine Zeile mit fehlendem Pflichtfeld
/// → [`SourceError::MissingMandatoryField`] (redigierter Feldname).
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn IngestedSourceSeam,
}

impl RawBookingSource for SourceBridge<'_> {
    fn read(&self, id: &BookingId) -> Result<IngestedBooking, SourceError> {
        match self.seam.read(id.as_str()) {
            None => Err(SourceError::NotFound(id.clone())),
            Some(row) => row
                .to_booking(id)
                .map_err(|missing| SourceError::MissingMandatoryField {
                    field: missing.field(),
                }),
        }
    }
}
