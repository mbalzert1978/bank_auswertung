//! Das Orakel des Laufs: der **sammelnde Kategorisierer**
//! ([Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md)).
//!
//! **Warum er sammelt statt antwortet.** Das Orakel ist ein Chat: die Anfragen gehen als Datei
//! hinaus, der Nutzer trägt sie selbst hinein, die Antwort kommt mit einem eigenen Befehl
//! (`kategorien-import`) zurück. Während des Laufs gibt es also **keine** Antwort — der Kategorisierer
//! merkt sich die Anfragen und antwortet leer. Die unbekannten Buchungsarten bleiben in diesem Lauf
//! unkategorisiert, und genau das ist der Lackmustest von
//! [ADR-0004](../../../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md): ohne das LLM
//! rechnet der Lauf alles, es fehlt allein das Etikett.
//!
//! **Gefragt wird er nur nach Unbekanntem** — den Abgleich mit dem Kategorie-Register macht die Domäne
//! vorher. Was er sammelt, ist darum genau der Inhalt des Chat-Exports.
//!
//! **Zwei Zustände statt Flag plus Liste.** *Ruhend* merkt er sich nichts — so lange Phase 1 eines
//! Laufs einen liegengebliebenen **alten** Lauf zu Ende führen kann, dessen Anfragen nicht in diesen
//! Export gehören. *Sammelnd* ist er ab Phase 2, die nur Nachrichten des eben geminteten Laufs
//! verarbeitet ([ADR-0016](../../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)).

use std::cell::RefCell;
use std::collections::BTreeMap;

use auswerten_adapter::{
    CategorizerSeam, CategorizerUnavailable, CategoryAnswerRow, CategoryRequestRow,
};

/// Was der Kategorisierer gerade tut. Die gesammelten Anfragen gibt es nur im Zustand *sammelnd* —
/// eine Liste neben einem Schalter, die im ruhenden Zustand trotzdem existierte, lässt dieser Typ
/// nicht zu.
#[derive(Debug, Default)]
enum Collection {
    /// Antwortet leer und merkt sich nichts.
    #[default]
    Resting,
    /// Antwortet leer und merkt sich je Fingerabdruck die **erste** Anfrage.
    Collecting(BTreeMap<String, CategoryRequestRow>),
}

/// Der sammelnde Kategorisierer (siehe Modul-Doku). Er reicht **nichts** hinaus — die Datei schreibt
/// erst der Export am Ende eines gelungenen Laufs, aus dem, was hier gesammelt wurde.
#[derive(Debug, Default)]
pub struct CollectingCategorizer {
    state: RefCell<Collection>,
}

impl CollectingCategorizer {
    /// Beginnt zu sammeln — der Lauf ruft es zu Beginn von Phase 2. Schon Gesammeltes verfällt.
    pub fn start_collecting(&self) {
        self.state.replace(Collection::Collecting(BTreeMap::new()));
    }

    /// Hört auf zu sammeln und gibt die Anfragen zurück, nach Fingerabdruck sortiert. Ruhte er,
    /// ist die Liste leer.
    #[must_use]
    pub fn take(&self) -> Vec<CategoryRequestRow> {
        match self.state.replace(Collection::Resting) {
            Collection::Resting => Vec::new(),
            Collection::Collecting(requests) => requests.into_values().collect(),
        }
    }
}

impl CategorizerSeam for CollectingCategorizer {
    /// Merkt sich die Anfragen (beim Sammeln) und antwortet **leer** — die Domäne verlangt keine
    /// vollständige Abdeckung ihrer Anfragen. Je Fingerabdruck gewinnt die **erste** Anfrage, wie in
    /// der Domäne je Fenster; die Fenster eines Laufs kommen in Outbox-Reihenfolge, für dieselbe Datei
    /// also immer gleich.
    fn ask(
        &self,
        requests: &[CategoryRequestRow],
    ) -> Result<Vec<CategoryAnswerRow>, CategorizerUnavailable> {
        if let Collection::Collecting(collected) = &mut *self.state.borrow_mut() {
            requests.iter().for_each(|request| {
                collected
                    .entry(request.fingerprint().to_string())
                    .or_insert_with(|| request.clone());
            });
        }
        Ok(Vec::new())
    }
}
