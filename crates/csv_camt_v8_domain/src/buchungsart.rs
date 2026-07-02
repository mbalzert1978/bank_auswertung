/// Die maschinelle Art einer Buchung (Treiber der deterministischen
/// Wiederholungs- und Gebühren-Erkennung). `Sonstige` fängt Buchungstexte ab,
/// die noch keiner der bekannten Arten zugeordnet sind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuchungsartEntity {
    Lastschrift,
    Dauerauftrag,
    Überweisung,
    Entgelt,
    Sonstige,
}

impl BuchungsartEntity {
    /// Deutet den rohen Buchungstext auf die Buchungsart — erkennt die bekannten
    /// Arten per Teilzeichenkette (case-insensitiv, inkl. der ASCII-Schreibweise
    /// `Ueberweisung`); alles Übrige ist `Sonstige`.
    pub fn aus_buchungstext(text: &str) -> Self {
        const PATTERNS: &[(&str, BuchungsartEntity)] = &[
            ("LASTSCHRIFT", BuchungsartEntity::Lastschrift),
            ("DAUERAUFTRAG", BuchungsartEntity::Dauerauftrag),
            ("ÜBERWEISUNG", BuchungsartEntity::Überweisung),
            ("UEBERWEISUNG", BuchungsartEntity::Überweisung),
            ("ENTGELT", BuchungsartEntity::Entgelt),
        ];
        let upper = text.to_uppercase();
        PATTERNS
            .iter()
            .find(|(pattern, _)| upper.contains(pattern))
            .map_or(BuchungsartEntity::Sonstige, |(_, kind)| *kind)
    }
}

#[cfg(test)]
mod tests {
    use super::BuchungsartEntity;

    #[test]
    fn bekannte_arten_werden_case_insensitiv_erkannt() {
        assert_eq!(
            BuchungsartEntity::aus_buchungstext("SEPA-Lastschrift"),
            BuchungsartEntity::Lastschrift
        );
        assert_eq!(
            BuchungsartEntity::aus_buchungstext("DAUERAUFTRAG"),
            BuchungsartEntity::Dauerauftrag
        );
        assert_eq!(
            BuchungsartEntity::aus_buchungstext("Ueberweisung"),
            BuchungsartEntity::Überweisung
        );
        assert_eq!(
            BuchungsartEntity::aus_buchungstext("Überweisung"),
            BuchungsartEntity::Überweisung
        );
        assert_eq!(
            BuchungsartEntity::aus_buchungstext("Kontofuehrungsentgelt"),
            BuchungsartEntity::Entgelt
        );
    }

    #[test]
    fn unbekannter_buchungstext_wird_sonstige() {
        assert_eq!(
            BuchungsartEntity::aus_buchungstext("Gutschrift"),
            BuchungsartEntity::Sonstige
        );
    }
}
