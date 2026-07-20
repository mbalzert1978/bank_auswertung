use crate::value_objects::NonEmptyString;

/// Die **Gegenpartei** einer Buchung, so wie sie stromabwärts der Obfuskierungs-Grenze ankommt —
/// der Summentyp, an dem `obfuskieren` die **Fail-safe-Deutung** Firma vs. Privatperson bereits
/// gefällt hat (CONTEXT.md „Firma vs. Privatperson"). Dieser Slice **liest** diese Deutung, er
/// führt sie nicht erneut: eine zweite Kopie derselben Regel könnte divergieren.
///
/// **Datenschutz:** keine Variante trägt einen **strukturellen Identifikator** — der Begriff meint
/// hier genau das, was CONTEXT.md („Obfuskierung") darunter fasst: eigene IBAN/Kontonr.,
/// Gegen-IBAN, BIC. Die fallen stromaufwärts vollständig weg.
///
/// Was eine Firma trägt, ist ihr **Klartext-Name** (verbatim durchgereicht — das wichtigste
/// Kategorie-Signal) und die **Gläubiger-ID**. Letztere ist keine *Ausnahme* von der Regel, sondern
/// liegt außerhalb ihrer: sie identifiziert die **Firma, nicht den Kontoinhaber**, und ist darum
/// als Analyse-Anker gewollt (ADR-0008). Eine Privatperson trägt **nur** ihr seed-basiertes
/// `PERSON_…`-Pseudonym; der Seed selbst bleibt in `obfuskieren` und erreicht diesen Slice nie
/// (ADR-0010).
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

impl Recipient {
    /// Das **Kategorie-Signal** der Gegenpartei: der Klartext-Firmenname, sonst nichts.
    ///
    /// Er ist es, weil er eine **Firma** benennt — `obfuskieren` reicht ihn genau dafür verbatim
    /// durch (CONTEXT.md „Empfänger": das wichtigste Signal für die Kategorie). Das Pseudonym einer
    /// Privatperson ist **keines**: `PERSON_7` sagt über die Art der Buchung nichts aus, und es
    /// hinauszureichen wäre Egress ohne jeden Gegenwert.
    pub(crate) fn category_signal(&self) -> Option<&NonEmptyString> {
        match self {
            Recipient::Company { name, .. } => name.as_ref(),
            Recipient::Person { .. } | Recipient::Absent => None,
        }
    }
}
