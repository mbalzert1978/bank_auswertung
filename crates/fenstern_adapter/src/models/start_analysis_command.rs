use application_kernel::{Command, CommandDecoder};

/// Der Dispatch-Key der **zweiten** vom Slice konsumierten Nachricht — auf ihn löst `io`s Prozessor
/// den Anstoß-Handler auf. Prosa-Name im ADR: der **Anstoß**.
///
/// Er steht neben [`CONSUMED_MESSAGE_TYPE`](crate::CONSUMED_MESSAGE_TYPE) (dem Fenster-Schritt),
/// weil der Slice **zwei** Kommandotypen konsumiert — je einen pro einzweckigem Handler (ADR-0013,
/// Entscheidung B/D).
pub const START_MESSAGE_TYPE: &str = "StartAnalysis";

/// Das **feldlose** Auslöser-Kommando am Slice-Eingang — derselbe Kickoff-Mechanismus wie bei
/// `einlesen`: ein **reiner Trigger**. Dem Slice ist egal, **wer** ihn einreiht (heute manuell,
/// später der Broker beim Leerlaufen des Imports).
///
/// Es trägt bewusst **keine** Nutzlast: der Anker wird nicht mitgegeben, sondern vom
/// [`StartHandler`](crate::StartHandler) über den Anker-Port beschafft. Damit ist der illegale
/// Zustand „leerer Anker" **unkonstruierbar** statt geprüft (ADR-0005) — anders als bei einem
/// einzelnen Kommando mit `Option<anchor>`. Es erfüllt `application_kernel::Command` (Marker).
pub struct StartAnalysisCommand;

impl Command for StartAnalysisCommand {}

/// Der slice-gelieferte [`CommandDecoder`] des Anstoßes (reine ACL, Shape Y). Weil das Kommando
/// feldlos ist, hat er **nichts zu deuten**: er nimmt jede Envelope an — `partition_key` und
/// `content` sind für einen reinen Trigger ohne Bedeutung, und es gibt keine Form, die er
/// zurückweisen könnte. Genau daran ist ablesbar, dass hier **kein** Anker über die Wire reist.
pub struct StartAnalysisCommandDecoder;

impl CommandDecoder for StartAnalysisCommandDecoder {
    type Command = StartAnalysisCommand;

    fn decode(_partition_key: &str, _content: &str) -> Option<Self::Command> {
        Some(StartAnalysisCommand)
    }
}
