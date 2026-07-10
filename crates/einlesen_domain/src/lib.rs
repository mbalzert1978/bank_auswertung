//! einlesen-Slice — **Domäne** (blind, ohne interne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md) /
//! [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)).
//!
//! Der Kopf der event-getriebenen Pipeline liest Rohzeilen ein und deutet sie in ein
//! `LineAggregate` aus Wert-Typen, die sich **selbst** aus dem `NonEmptyString` parsen,
//! den die Domäne an ihrer Grenze anfordert (parse, don't validate) und dabei `Result`
//! liefern; reine Format-Mechanik (Kalenderdatum, deutsches Zahlenformat) liegt dagegen
//! hinter den verlangten Ports `Calendar`/`Money` (ADR-0008). Das Aggregat orchestriert
//! seine Wert-Typen und kennt seine Grenzen (eine Zeile). Die Rohdaten **beschafft** der
//! Handler als `RawLine`-Batch über den verlangten Datenquellen-Port [`RawLineSource`]
//! ([Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md);
//! das Kommando ist reiner Auslöser) — **wie** eine Zeile adressiert und gelesen wird,
//! ist Sache des Adapters.
//!
//! **Roh & obfuskierungs-frei:** die Domäne trägt rohe Werte (IBANs, Klarname)
//! unverändert weiter — das Obfuskieren ist der nachgelagerte `obfuskieren`-Slice,
//! die Eigen/Fremd-Auflösung der `auswerten`-Slice. Datenschutz: die Domäne bindet
//! **kein** serde; die rohen Werte reisen nicht mehr per Outbox-Nachricht, sondern
//! **in-process** über den Quell-Seam des Adapters in die Domäne (und liegen lokal in
//! der Roh-Tabelle) — kein Egress.
//!
//! **Aufbau:** die Wert-Objekte liegen in `value_objects/`, die von der Domäne
//! verlangten Ports in `ports/`, das Kommando samt Nutzlast in `commands/`, das
//! Zeilen-Aggregat in `aggregate/`; `lib.rs` zieht die öffentlichen Typen auf
//! Modulebene, sodass Konsumenten `einlesen_domain::Amount` schreiben, nicht
//! `einlesen_domain::value_objects::Amount`.

mod aggregate;
mod commands;
mod error;
mod handler;
mod ports;
mod value_objects;

pub use aggregate::LineAggregate;
pub use commands::{RawLine, ReadTransactions, RunConfiguration};
pub use error::{LineError, ReadError, SourceError};
pub use handler::ReadHandler;
pub use ports::{Calendar, EventSink, Money, RawLineSource, Store};
pub use value_objects::{
    CalendarDay, Currency, DecodedAmount, Direction, IngestionId, NonEmptyString, Sign,
};
