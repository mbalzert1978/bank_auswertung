use application_kernel::{Command, PublishCommand};

use crate::models::advance_window_command::CONSUMED_MESSAGE_TYPE;

/// Das ausgehende **Publish-Command** der Selbst-Fortsetzung: derselbe Nachrichtentyp, den der Slice
/// auch **konsumiert** (`AdvanceWindow`) — der Fenster-Schritt reiht sein eigenes nächstes Kommando
/// ein, bis die Historie versiegt (ADR-0013, Entscheidung B).
///
/// Es trägt denselben `partition_key` wie das eingehende Kommando — `"{anchor}:{run_top}:{run}"`,
/// **kein serde**. Der `run_top` reist unverändert durch den ganzen Lauf, damit der terminierende Schritt
/// die **Lauf-Spanne** kennt, ohne Zustand zu halten oder erneut zu queren.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvanceWindowMessage {
    /// Der **ganze** Schlüssel `"{anchor}:{run_top}:{run}"`, nicht nur der Anker — er wird als Einheit
    /// gebaut und als Einheit gelesen; wer nur den Anker daraus will, muss ihn decodieren
    /// (`AdvanceWindowCommandDecoder`).
    partition_key: String,
}

impl AdvanceWindowMessage {
    /// Zeigt auf die nächste Seite: ihr Anker, der unveränderte Lauf-Gipfel und der Lauf.
    pub(crate) fn from_anchor(anchor: i64, run_top: i64, run: &str) -> Self {
        Self {
            partition_key: format!("{anchor}:{run_top}:{run}"),
        }
    }
}

impl Command for AdvanceWindowMessage {}

impl PublishCommand for AdvanceWindowMessage {
    const MESSAGE_TYPE: &'static str = CONSUMED_MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.partition_key
    }
}
