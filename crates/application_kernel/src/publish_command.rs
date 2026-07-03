use crate::Command;

/// Ein Kommando, das über einen [`Publisher`](crate::Publisher) zur späteren
/// Verarbeitung versendet werden kann. [`MESSAGE_TYPE`](PublishCommand::MESSAGE_TYPE)
/// ist der selbst-identifizierende Dispatch-Key, auf den die (später folgende)
/// Messaging-Schicht den passenden Handler auflöst — die eine Quelle des Keys,
/// analog zur C#-Referenz.
pub trait PublishCommand: Command {
    /// Der Dispatch-Key dieses Kommandotyps. **Assoziierte Konstante statt
    /// Methode:** der Key ist eine kompilierzeit-feste Eigenschaft des Typs
    /// (`C::MESSAGE_TYPE`), kein Verhalten — die idiomatische Rust-Form dafür.
    const MESSAGE_TYPE: &'static str;
}
