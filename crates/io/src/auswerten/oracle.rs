//! Die beiden **Platzhalter** der Kategorisierung: das Register, das noch nichts weiß, und das Orakel,
//! das noch nicht angeschlossen ist.
//!
//! **Warum es sie gibt.** Der `auswerten`-Slice verlangt beide Seams; gebaut sind sie nicht — das
//! persistente Kategorie-Register ist
//! [Issue 0009](../../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md),
//! der Orakel-Adapter (Export → Chat → Import)
//! [Issue 0011](../../../../docs/issues/0011-kategorisierer-chat-payload-adapter.md). Ohne eine
//! Erfüllung könnte ein realer Lauf nicht laufen. Diese beiden sind darum **echte** Adapter mit einer
//! bewusst leeren Antwort — keine Test-Fakes.
//!
//! **Und genau deshalb sind sie unbedenklich:** die Domäne **ist** die Analyse, das LLM nur ein Orakel
//! ([ADR-0004](../../../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)). Der
//! Lackmustest dieses ADR lautet „ohne LLM bleibt das Programm wertvoll" — hier wird er eingelöst:
//! der Lauf rechnet Cashflow, Fingerabdrücke und Eigen/Fremd vollständig, es fehlt allein das
//! **Etikett** für unbekannte Buchungsarten. Ein Fehlschlag statt einer leeren Antwort wäre die
//! falsche Lesart: er machte das Orakel zur Voraussetzung der Analyse.

use auswerten_adapter::{
    CategorizerSeam, CategorizerUnavailable, CategoryAnswerRow, CategoryRegistrySeam,
    CategoryRequestRow, RegistryUnavailable,
};

/// Das **noch leere** Kategorie-Register: es kennt keinen Fingerabdruck, also ist jede Buchungsart
/// unbekannt. Erreichbar (nie `Err`) — ein Register, das es noch nicht gibt, ist nicht dasselbe wie
/// eines, das ausgefallen ist, und nur Letzteres darf einen Lauf beenden.
///
/// Es tritt ab, sobald
/// [Issue 0009](../../../../docs/issues/0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
/// die persistente Ablage samt gelernten Korrekturen baut.
#[derive(Debug, Default)]
pub struct EmptyCategoryRegistry;

impl CategoryRegistrySeam for EmptyCategoryRegistry {
    /// Keiner der Kandidaten ist bekannt — die leere Teilmenge.
    fn known_among(&self, _candidates: &[String]) -> Result<Vec<String>, RegistryUnavailable> {
        Ok(Vec::new())
    }
}

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
