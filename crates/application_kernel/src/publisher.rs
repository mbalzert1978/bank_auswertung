use crate::PublishCommand;

/// Domänenseitiger Port, um ein [`PublishCommand`] zu veröffentlichen. Die Domäne
/// spricht nur ihr eigenes Kommando-Vokabular; eine Bridge im Adapter setzt es
/// später auf den konkreten Transport (Outbox) um. Genau diese Indirektion
/// erlaubt es einem Slice-Test, einen Fake-Publisher unterzuschieben und
/// veröffentlichte Kommandos zu beobachten.
pub trait Publisher<C: PublishCommand> {
    /// Veröffentlicht das Kommando.
    fn publish(&self, command: C);
}
