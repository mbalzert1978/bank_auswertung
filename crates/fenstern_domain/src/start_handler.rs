use crate::error::WindowError;
use crate::models::{AdvanceWindow, StartAnalysis};
use crate::ports::{AdvanceWindowPublisher, WindowAnchor};

/// Der **zweite einzweckige Handler** des Slice — der **Anstoß**
/// ([ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung B/D). Er hat genau **eine** Aufgabe: den Lauf eröffnen. Das Fenster selbst
/// materialisiert der [`WindowingHandler`](crate::WindowingHandler), den das hier publizierte
/// Kommando erst auslöst.
///
/// **Ablauf je Kommando** — zwei Schritte, kein dritter:
/// 1. den Initial-Anker **einmalig** über den [`WindowAnchor`]-Port lesen (`max(booking_day)`,
///    also **datenrelativ** und deterministisch — keine Uhr),
/// 2. das erste [`AdvanceWindow`] mit diesem Anker publizieren.
///
/// Der Anker ist dabei zugleich der **Lauf-Gipfel**: der Lauf beginnt an seinem jüngsten Tag, und
/// genau dieser Wert reist unverändert durch die ganze Selbst-Fortsetzung, bis der terminierende
/// Schritt daraus die Lauf-Spanne bildet.
///
/// **Der leere Datenbestand ist kein Fehler.** Liefert der Anker-Port keinen Buchungstag, endet der
/// Lauf mit **Erfolg**: es wird **nichts** publiziert und `auswerten` **nicht** angestoßen —
/// Erschöpfung schon bei Schritt 0, symmetrisch zum normalen Ketten-Ende. Kein Dead-Letter (den
/// gibt es nicht), kein „nichts auszuwerten"-Fehler (ADR-0012). Nebeneffekt, der zählt: weil so
/// **kein** `HistoryComplete` mit leerer Spanne entsteht, bleibt dessen Lauf-Spanne garantiert nicht
/// entartet.
///
/// Wie sein Geschwister ist er **transaktions-blind**: `publish` *staged* nur, den atomaren Commit
/// besorgt io. Lesen und Publizieren sind je fehlbar und terminal; ihr [`WindowError`] propagiert
/// per `?` — kein Retry im Slice.
pub struct StartHandler<'a> {
    anchors: &'a dyn WindowAnchor,
    advance: &'a dyn AdvanceWindowPublisher,
}

impl<'a> StartHandler<'a> {
    pub fn new(anchors: &'a dyn WindowAnchor, advance: &'a dyn AdvanceWindowPublisher) -> Self {
        Self { anchors, advance }
    }

    /// Eröffnet den Lauf: Anker lesen → erstes Fenster-Kommando publizieren. `Ok(())` = verarbeitet
    /// (auch beim leeren Bestand, dann ohne Publish); `Err(WindowError)` = terminaler Fehler.
    pub fn handle(&self, command: StartAnalysis) -> Result<(), WindowError> {
        match self.anchors.latest()? {
            // Der jüngste Buchungstag eröffnet den Lauf — als Anker **und** als Lauf-Gipfel; die
            // Lauf-Identität des Anstoßes reist von hier an mit (ADR-0016).
            Some(top) => self
                .advance
                .publish(&AdvanceWindow::new(top, top, command.run().clone())),
            // Leerer Bestand: erfolgreich, aber ohne jede Nachricht.
            None => Ok(()),
        }
    }
}
