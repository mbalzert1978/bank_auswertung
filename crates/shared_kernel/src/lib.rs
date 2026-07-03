//! Shared-Kernel — die **dünne**, interface-only Cross-Cutting-Schicht des
//! modularen Monolithen ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
//!
//! Hier liegen **nur Verträge** (Traits/Marker), die alle Adapter-Layer teilen —
//! keine Value Objects, keine Geschäftsdomäne. Der Kernel läuft bewusst **nicht**
//! auf der Domäne zusammen (sonst wären die Slices wieder gekoppelt) und hat
//! **keine** Dependencies. Genutzt werden diese Traits erst von den Adaptern,
//! sobald das Messaging (Commands/Publisher/Outbox) kommt — jetzt sind sie
//! ungewired und stehen nur als Konvention bereit. Vorbild: die C#-Referenz
//! `Common/Abstractions` (`IHandler`/`IPublishCommand`/`IPublisher`).

/// Marker für ein Kommando in Domänenvokabular. Ein Kommando ist die eine
/// Eingabe, die ein [`Handler`] verarbeitet.
pub trait Command {}

/// Ein Kommando, das über einen [`Publisher`] zur späteren Verarbeitung versendet
/// werden kann. [`message_type`](PublishCommand::message_type) ist der
/// selbst-identifizierende Dispatch-Key, auf den die (später folgende)
/// Messaging-Schicht den passenden Handler auflöst — die eine Quelle des Keys,
/// analog zur C#-Referenz.
pub trait PublishCommand: Command {
    /// Der Dispatch-Key dieses Kommandotyps (stabil, typgebunden).
    fn message_type() -> &'static str;
}

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
    /// ist ein [`PublishCommand`] und trägt dort den Dispatch-Key. Eine `Response`
    /// darf ein rein interner Domänen-Wert bleiben (das C#-`IResponse`-Marker-
    /// Interface hat hier keine Entsprechung, weil der Constraint anderswo sitzt).
    type Response;
    /// Der fachliche Fehler, mit dem die Verarbeitung scheitern kann. Schranke
    /// `std::error::Error`, damit ihn jede Pipeline-Stufe ohne Wiederholung von
    /// `where`-Klauseln als Fehler behandeln kann; Domänenfehler erfüllen sie
    /// ohnehin (`Display` + `std::error::Error`, [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
    type Error: std::error::Error;

    /// Verarbeitet das Kommando — oder scheitert mit [`Self::Error`].
    fn handle(&self, command: Self::Command) -> Result<Self::Response, Self::Error>;
}

/// Domänenseitiger Port, um ein [`PublishCommand`] zu veröffentlichen. Die Domäne
/// spricht nur ihr eigenes Kommando-Vokabular; eine Bridge im Adapter setzt es
/// später auf den konkreten Transport (Outbox) um. Genau diese Indirektion
/// erlaubt es einem Slice-Test, einen Fake-Publisher unterzuschieben und
/// veröffentlichte Kommandos zu beobachten.
pub trait Publisher<C: PublishCommand> {
    /// Veröffentlicht das Kommando.
    fn publish(&self, command: C);
}
