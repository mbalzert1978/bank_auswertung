use crate::{Fingerabdruck, ObfuskierterText};

/// Die schmale, eigene Ausgangs-DTO des `Kategorisierer`-Ports (kein
/// Gott-DTO): Fingerabdruck + obfuskierter Freitext für Empfänger und
/// Verwendungszweck. Enthält strukturell keinen Platz für einen
/// strukturellen Identifikator (`Iban`/`Bic`) — siehe
/// `strukturelle_identifikatoren` für den compilergeprüften Nachweis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct KategorieAnfrage {
    pub fingerabdruck: Fingerabdruck,
    pub empfänger: ObfuskierterText,
    pub verwendungszweck: ObfuskierterText,
}
