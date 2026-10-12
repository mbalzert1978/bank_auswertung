//! Der **Platzhalter** des Orakels: der Kategorisierer, der noch nicht angeschlossen ist.
//!
//! **Warum es ihn gibt.** Der `auswerten`-Slice verlangt den Orakel-Seam; der Orakel-Adapter
//! (Export → Chat → Import) ist
//! [Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md). Ohne eine
//! Erfüllung könnte ein realer Lauf nicht laufen. Dieser Adapter ist darum ein **echter** Adapter mit
//! einer bewusst leeren Antwort — kein Test-Fake. Sein früherer Partner, das leere Register, ist mit
//! [Issue 0009](../../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
//! dem realen [`SqliteCategoryRegistry`](crate::auswerten::SqliteCategoryRegistry) gewichen: gefragt
//! wird dieser Platzhalter seitdem nur noch für Arten, die das Register **nicht** kennt.
//!
//! **Und genau deshalb ist er unbedenklich:** die Domäne **ist** die Analyse, das LLM nur ein Orakel
//! ([ADR-0004](../../../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)). Der
//! Lackmustest dieses ADR lautet „ohne LLM bleibt das Programm wertvoll" — hier wird er eingelöst:
//! der Lauf rechnet Cashflow, Fingerabdrücke und Eigen/Fremd vollständig, es fehlt allein das
//! **Etikett** für unbekannte Buchungsarten. Ein Fehlschlag statt einer leeren Antwort wäre die
//! falsche Lesart: er machte das Orakel zur Voraussetzung der Analyse.

use auswerten_adapter::{
    CategorizerSeam, CategorizerUnavailable, CategoryAnswerRow, CategoryRequestRow,
};

/// Das **noch nicht angeschlossene** Orakel: es nimmt die Anfragen entgegen und antwortet mit
/// **keinem** Etikett. Die unbekannten Buchungsarten bleiben damit schlicht unkategorisiert, statt den
/// Lauf zu beenden (ADR-0004, siehe Modul-Doku).
///
/// **Kein Egress:** dieser Adapter reicht nichts hinaus — die Anfragen verlassen die Maschine
/// nirgendwohin. Genau das ändert
/// [Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md), und **dann**
/// wird die compilerfeste „kein `Serialize`"-Sperre der `CategoryRequestRow` scharf.
#[derive(Debug, Default)]
pub struct UnansweredCategorizer;

impl CategorizerSeam for UnansweredCategorizer {
    /// Keine Antwort — die Domäne verlangt keine vollständige Abdeckung ihrer Anfragen.
    fn ask(
        &self,
        _requests: &[CategoryRequestRow],
    ) -> Result<Vec<CategoryAnswerRow>, CategorizerUnavailable> {
        Ok(Vec::new())
    }
}
