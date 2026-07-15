//! Die **Entities** des Slice — über ihre **Identität** definierte Typen: in der DB
//! abgelegt und per (Store-vergebener) ID auffindbar. Das Zeilen-Aggregat ist der Kern
//! des Slice: es orchestriert die Wert-Objekte und validiert eine ganze Rohzeile an
//! *einem* Ort.

mod line_aggregate;

pub use line_aggregate::LineAggregate;
