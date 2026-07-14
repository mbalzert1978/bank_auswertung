use crate::value_objects::{BookingId, IngestedBooking};

/// Der von der Domäne **verlangte** Lese-Port, über den der Handler das rohe Aggregat einer Buchung
/// **per ID** aus dem **einlesen-Speicher** (Roh-Tabelle) holt. **io erfüllt ihn** (liest die Zeile
/// und deutet sie in ein [`IngestedBooking`] der Obfuskierungs-Domäne); die Persistenz dahinter ist
/// vertagt, ein Fake trägt ihn in den Specs. So trägt die Outbox-Nachricht nur die **ID** (Zeiger),
/// die Nutzlast liegt im Speicher.
pub trait RawBookingSource {
    /// Liest das rohe Aggregat zur ID — **dreiwertig** ([`BookingLookup`]): vorhanden, abwesend
    /// (gebrochenes Invariant, terminal) oder vorhanden-aber-ungültig (terminal, redigierter
    /// Feldname). Die Quelle muss eine fehlende Zeile **melden** können; ihre Disposition (überall
    /// terminal) besitzt `io`, nicht dieser Port.
    fn read(&self, id: &BookingId) -> BookingLookup;
}

/// Das **dreiwertige** Ergebnis eines Lese-Versuchs — die drei Ausgänge liegen im **Typ**, nicht in
/// einem überladenen `Option::None`, damit die Quelle „abwesend" von „ungültig" unterscheiden kann.
/// Beide Fehler-Ausgänge sind **terminal**; die Trennung bleibt im Typ, damit `io` je einen eigenen
/// redigierten Grund spiegelt (opake ID vs. statischer Feldname).
#[derive(Debug)]
pub enum BookingLookup {
    /// Zur ID liegt ein deutbares rohes Aggregat vor. **Geboxt**, weil das Aggregat weit größer ist
    /// als die schlanken Fehler-Varianten (`clippy::large_enum_variant`).
    Found(Box<IngestedBooking>),
    /// Zur ID liegt **keine** Zeile vor — ein **gebrochenes Invariant** (terminal): `einlesen`
    /// committet Roh-Zeile **und** `Ingested`-Nachricht atomar in **einer** `UnitOfWork`, und der
    /// Broker stellt eine Nachricht erst **nach** dem Commit ihrer erzeugenden Transaktion zu. Sieht
    /// der Slice den Zeiger, **liegt** die Zeile also im Speicher; ihre Abwesenheit heißt korrupter
    /// Speicher / falsche ID — der Adapter mappt das auf Dead-Letter (kein Retry).
    Absent,
    /// Zur ID liegt eine Zeile vor, aber ein **Pflichtfeld fehlt/ist leer** — **terminal**
    /// (Dead-Letter). Trägt **nur** den statischen Feldnamen (redigiert, nie ein Rohwert).
    Malformed { field: &'static str },
}
