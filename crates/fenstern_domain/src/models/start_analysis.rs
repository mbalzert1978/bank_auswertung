use crate::value_objects::RunId;

/// Das domänen-interne **Auslöser-Kommando** des Anstoßes
/// ([ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung B/D). Dem Slice ist egal, **wer** es einreiht; es trägt **keinen Anker** — der wird
/// nicht mitgegeben, sondern vom [`StartHandler`](crate::StartHandler) über den
/// [`WindowAnchor`](crate::WindowAnchor)-Port **beschafft**.
///
/// Es trägt allein die **Identität des Laufs**, den es eröffnet
/// ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)): sie reist von
/// hier durch jedes Fenster-Kommando bis in jedes Fenster und den Abschluss. Wer anstößt, vergibt sie
/// — der Slice deutet sie nie.
///
/// Warum **zwei** Kommandos und nicht eines mit optionalem Anker: der **Typ trägt die Absicht**.
/// Dieses Kommando startet, [`AdvanceWindow`](crate::AdvanceWindow) trägt **garantiert** einen Anker;
/// der illegale Zustand „leerer Anker" ist damit **unkonstruierbar** statt geprüft (ADR-0005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartAnalysis {
    run: RunId,
}

impl StartAnalysis {
    #[must_use]
    pub fn new(run: RunId) -> Self {
        Self { run }
    }

    /// Der Lauf, den dieser Anstoß eröffnet.
    #[must_use]
    pub fn run(&self) -> &RunId {
        &self.run
    }
}
