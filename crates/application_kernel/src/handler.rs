use crate::Command;

/// Verarbeitet genau einen Kommandotyp zu einer Antwort. Der Handler lebt in der
/// Slice-Domäne und hängt nur an deren Ports; die Verdrahtung an die Außenwelt
/// besorgt der Adapter. Synchron, weil die Auswertung ein Batch-Lauf ist (kein
/// async-Runtime, Datenschutz-Bonus — [ADR-0003](../../../docs/adr/0003-rust-als-sprache.md)).
pub trait Handler {
    /// Das Kommando, das dieser Handler entgegennimmt.
    type Command: Command;
    /// Das Ergebnis der Verarbeitung. **Bewusst ohne Schranke:** die
    /// Weiterverarbeitbarkeit in der Pipeline (Serialisierung in die nächste
    /// Outbox-Message) wird nicht auf der `Response` erzwungen, sondern an der
    /// Publisher-/Command-Grenze — nur das, was tatsächlich weiterversendet wird,
    /// ist ein [`PublishCommand`](crate::PublishCommand) und trägt dort den
    /// Dispatch-Key. Eine `Response` darf ein rein interner Domänen-Wert bleiben
    /// (das C#-`IResponse`-Marker-Interface hat hier keine Entsprechung, weil der
    /// Constraint anderswo sitzt).
    type Response;
    /// Der fachliche Fehler, mit dem die Verarbeitung scheitern kann. Schranke
    /// `std::error::Error`, damit ihn jede Pipeline-Stufe ohne Wiederholung von
    /// `where`-Klauseln als Fehler behandeln kann; Domänenfehler erfüllen sie
    /// ohnehin (`Display` + `std::error::Error`, [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
    type Error: std::error::Error;

    /// Verarbeitet das Kommando — oder scheitert mit [`Self::Error`].
    fn handle(&self, command: Self::Command) -> Result<Self::Response, Self::Error>;
}
