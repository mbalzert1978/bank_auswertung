use obfuskierung_domain::{BookingId, IngestedBooking, RawBookingSource};

use crate::abstractions::IngestedSourceSeam;

/// Die **Port-Bridge** für den Lese-Port: erfüllt `obfuskierung_domain::RawBookingSource` und mappt
/// ihn auf den öffentlichen [`IngestedSourceSeam`] (den io erfüllt). „außen → innen": der Seam
/// liefert die rohe [`IngestedRow`](crate::dto::IngestedRow), die Bridge deutet sie in das
/// Domänen-Aggregat [`IngestedBooking`]. Vorbild: `CalendarBridge`/`MoneyBridge` des Einlese-Slice.
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn IngestedSourceSeam,
}

impl RawBookingSource for SourceBridge<'_> {
    fn read(&self, id: &BookingId) -> Option<IngestedBooking> {
        self.seam
            .read(id.as_str())
            .and_then(|row| row.to_booking(id.clone()))
    }
}
