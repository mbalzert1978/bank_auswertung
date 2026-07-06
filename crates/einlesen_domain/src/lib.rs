//! einlesen-Slice — **Domäne** (blind, ohne interne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md) /
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
//!
//! Der Kopf der event-getriebenen Pipeline liest Rohzeilen ein und deutet sie in ein
//! `ZeilenAggregat` aus Wert-Typen, die sich **selbst** aus dem `NonEmptyString` parsen,
//! den die Domäne an ihrer Grenze anfordert (parse, don't validate) und dabei `Result`
//! liefern; das Aggregat orchestriert seine Wert-Typen und kennt seine Grenzen (eine
//! Zeile). Die Rohdaten bezieht es über den Port `Rohzeile` — **wie** eine Zeile
//! adressiert wird, ist Sache des Adapters.
//!
//! **Roh & obfuskierungs-frei:** die Domäne trägt rohe Werte (IBANs, Klarname)
//! unverändert weiter — das Obfuskieren ist der nachgelagerte `obfuskieren`-Slice,
//! die Eigen/Fremd-Auflösung der `auswerten`-Slice. Datenschutz: die Domäne bindet
//! **kein** serde; die rohen Werte reisen erst im Adapter (serialisierbares Command)
//! durch die lokale Outbox — kein Egress.
//!
//! **Aufbau:** die Wert-Objekte liegen in `value_objects/`, die von der Domäne
//! verlangten Ports in `ports/`, das Zeilen-Aggregat in `aggregate/`; `lib.rs` zieht die
//! öffentlichen Typen auf Modulebene, sodass Konsumenten `einlesen_domain::Betrag`
//! schreiben, nicht `einlesen_domain::value_objects::Betrag`.

mod aggregate;
mod fehler;
mod ports;
mod value_objects;

pub use aggregate::ZeilenAggregat;
pub use fehler::ZeilenError;
pub use ports::{Kalender, Rohzeile};
pub use value_objects::{
    Betrag, Buchungstext, Datum, Empfänger, Empfängername, GläubigerId, Iban, Kalendertag,
    NonEmptyString, Richtung, Verwendungszweck, Währung,
};
