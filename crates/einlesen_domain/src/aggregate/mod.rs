//! Das Zeilen-Aggregat — der Kern des Slice: es orchestriert die Wert-Objekte und
//! validiert eine ganze Rohzeile an *einem* Ort.

mod line_aggregate;

pub use line_aggregate::LineAggregate;
