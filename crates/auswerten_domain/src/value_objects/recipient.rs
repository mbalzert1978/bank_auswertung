use crate::value_objects::NonEmptyString;

/// Die **Gegenpartei** einer Buchung, so wie sie stromabwärts der Obfuskierungs-Grenze ankommt —
/// der Summentyp, an dem `obfuskieren` die **Fail-safe-Deutung** Firma vs. Privatperson bereits
/// gefällt hat (CONTEXT.md „Firma vs. Privatperson"). Dieser Slice **liest** diese Deutung, er
/// führt sie nicht erneut: eine zweite Kopie derselben Regel könnte divergieren.
///
/// **Datenschutz:** keine Variante trägt einen strukturellen Identifikator. Eine Firma trägt ihren
/// Klartext-Namen und die **Gläubiger-ID** — den öffentlichen Firmenschlüssel, der als
/// Analyse-Anker eine bewusste Ausnahme ist (ADR-0008). Eine Privatperson trägt **nur** ihr
/// seed-basiertes `PERSON_…`-Pseudonym; der Seed selbst bleibt in `obfuskieren` und erreicht diesen
/// Slice nie (ADR-0010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recipient {
    /// Sichere Firma (Gläubiger-ID vorhanden): Name im Klartext (falls geführt) + Anker.
    Company {
        name: Option<NonEmptyString>,
        creditor_id: NonEmptyString,
    },
    /// Fail-safe Privatperson: **nur** das seed-basierte Pseudonym.
    Person { pseudonym: NonEmptyString },
    /// Keine Gegenpartei — z. B. eine Entgeltzeile.
    Absent,
}
