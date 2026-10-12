use application_kernel::{Command, CommandDecoder};
use fenstern_domain::{NonEmptyString, RunId};

/// Der Dispatch-Key der **zweiten** vom Slice konsumierten Nachricht — auf ihn löst `io`s Prozessor
/// den Anstoß-Handler auf. Prosa-Name im ADR: der **Anstoß**.
///
/// Er steht neben [`CONSUMED_MESSAGE_TYPE`](crate::CONSUMED_MESSAGE_TYPE) (dem Fenster-Schritt),
/// weil der Slice **zwei** Kommandotypen konsumiert — je einen pro einzweckigem Handler (ADR-0013,
/// Entscheidung B/D).
pub const START_MESSAGE_TYPE: &str = "StartAnalysis";

/// Das Auslöser-Kommando am Slice-Eingang — derselbe Kickoff-Mechanismus wie bei `einlesen`: ein
/// **Trigger**. Dem Slice ist egal, **wer** ihn einreiht.
///
/// Es trägt bewusst **keinen Anker**: der wird nicht mitgegeben, sondern vom
/// [`StartHandler`](crate::StartHandler) über den Anker-Port beschafft. Damit ist der illegale
/// Zustand „leerer Anker" **unkonstruierbar** statt geprüft (ADR-0005). Es trägt allein die
/// **Lauf-Identität**, die wer anstößt vergibt
/// ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)). Es erfüllt
/// `application_kernel::Command` (Marker).
pub struct StartAnalysisCommand {
    run: RunId,
}

impl StartAnalysisCommand {
    /// Der Lauf, den dieser Anstoß eröffnet.
    pub(crate) fn run(&self) -> &RunId {
        &self.run
    }
}

impl Command for StartAnalysisCommand {}

/// Der slice-gelieferte [`CommandDecoder`] des Anstoßes (reine ACL, Shape Y). Die Lauf-Identität
/// reist als **ganzer `partition_key`** (opak, kein serde); der `content` ist leer. Ein leerer oder
/// blanker Schlüssel benennt keinen Lauf → `None`, das `io` auf der Envelope-Ebene behandelt — ein
/// Fenster ohne Lauf wäre genau der Zustand, den ADR-0016 abschafft.
pub struct StartAnalysisCommandDecoder;

impl CommandDecoder for StartAnalysisCommandDecoder {
    type Command = StartAnalysisCommand;

    fn decode(partition_key: &str, _content: &str) -> Option<Self::Command> {
        NonEmptyString::new(partition_key).map(|run| StartAnalysisCommand {
            run: RunId::from(run),
        })
    }
}
