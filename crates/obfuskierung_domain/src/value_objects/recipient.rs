use crate::value_objects::replacement::{CreditorId, ObfuscatedText, Pseudonym};

/// Die obfuskierte Gegenpartei einer Buchung — der Summentyp, an dem die **Fail-safe-Deutung**
/// Firma vs. Privatperson fällt ([ADR-0005](../../../docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md);
/// CONTEXT.md „Firma vs. Privatperson"). Illegale Kombinationen sind unkonstruierbar: eine Person
/// trägt **nur** ein Pseudonym, eine Firma **nie** eines.
///
/// **Heuristik (fail-safe):** eine vorhandene Gläubiger-ID beweist sicher eine Firma → ihr Name
/// bleibt **Klartext** (als [`ObfuscatedText`], das wichtigste Kategorie-Signal), die
/// Gläubiger-ID bleibt Anker. Fehlt die Gläubiger-ID, ist Firma **nicht** sicher → im Zweifel
/// Privatperson: der Name wird **pseudonymisiert** (`PERSON_…`). Fehlt auch der Name, gibt es
/// keine Gegenpartei (Gebühren-/Entgeltzeile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recipient {
    /// Sichere Firma (Gläubiger-ID vorhanden): Name im Klartext (falls geführt) + Anker.
    Company {
        name: Option<ObfuscatedText>,
        creditor_id: CreditorId,
    },
    /// Fail-safe Privatperson (keine Gläubiger-ID, aber ein Name): pseudonymisiert.
    Person { pseudonym: Pseudonym },
    /// Keine Gegenpartei (weder Name noch Gläubiger-ID) — z. B. eine Entgeltzeile.
    Absent,
}
