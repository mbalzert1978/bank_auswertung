/// Das domänen-interne **Auslöser-Kommando** des Anstoßes: **feldlos** — derselbe
/// Kickoff-Mechanismus wie bei `einlesen`
/// ([ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung B/D). Dem Slice ist egal, **wer** es einreiht (heute manuell, später der Broker beim
/// Leerlaufen des Imports); es trägt keine Nutzlast, weil es keine braucht: der Anker wird nicht
/// mitgegeben, sondern vom [`StartHandler`](crate::StartHandler) über den
/// [`WindowAnchor`](crate::WindowAnchor)-Port **beschafft**.
///
/// Warum **zwei** Kommandos und nicht eines mit optionalem Anker: der **Typ trägt die Absicht**.
/// Dieses feldlose Kommando startet, [`AdvanceWindow`](crate::AdvanceWindow) trägt **garantiert**
/// einen Anker; der illegale Zustand „leerer Anker" ist damit **unkonstruierbar** statt geprüft
/// (ADR-0005). Jeder Handler hat **eine** Aufgabe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartAnalysis;
