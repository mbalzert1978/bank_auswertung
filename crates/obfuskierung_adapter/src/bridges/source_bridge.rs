use obfuskierung_domain::{BookingId, BookingLookup, RawBookingSource};

use crate::abstractions::IngestedSourceSeam;

/// Die **Port-Bridge** für den Lese-Port: erfüllt `obfuskierung_domain::RawBookingSource` und mappt
/// ihn auf den öffentlichen [`IngestedSourceSeam`] (den io erfüllt). „außen → innen": der Seam
/// liefert die rohe [`IngestedRow`](crate::dto::IngestedRow), die Bridge deutet sie in das
/// Domänen-Aggregat. Sie **trennt die drei Ausgänge im Typ** ([`BookingLookup`]): der Seam liefert
/// keine Zeile → `Absent` (gebrochenes Invariant, terminal — der Broker stellt erst nach `einlesen`s
/// atomarem Commit zu, die Zeile muss also liegen); die Zeile ist da, aber ein Pflichtfeld fehlt →
/// `Malformed` mit redigiertem Feldnamen (terminal); sonst `Found`. Vorbild: `CalendarBridge` des
/// Einlese-Slice.
pub(crate) struct SourceBridge<'a> {
    pub(crate) seam: &'a dyn IngestedSourceSeam,
}

impl RawBookingSource for SourceBridge<'_> {
    fn read(&self, id: &BookingId) -> BookingLookup {
        match self.seam.read(id.as_str()) {
            None => BookingLookup::Absent,
            Some(row) => match row.to_booking(id) {
                Ok(booking) => BookingLookup::Found(Box::new(booking)),
                Err(missing) => BookingLookup::Malformed {
                    field: missing.field(),
                },
            },
        }
    }
}
