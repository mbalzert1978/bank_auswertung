use super::NonEmptyString;

/// Die **Gläubiger-Identifikation** (SEPA Creditor ID) einer Firma — der eine
/// strukturelle Identifikator, der die Maschine verlassen darf: eine öffentliche
/// Firmenkennung, bewusst als Analyse-Anker der Abo-Erkennung bewahrt (CONTEXT.md,
/// ADR-0001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditorId(NonEmptyString);

impl From<NonEmptyString> for CreditorId {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

impl CreditorId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Der (Klartext-)Name des Empfängers — das wichtigste Kategorie-Signal (CONTEXT.md).
/// In der Einlese-Stufe bewusst **roh** getragen; das Pseudonymisieren einer
/// Privatperson ist Sache des nachgelagerten `obfuskieren`-Slice, nicht dieser Stufe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipientName(NonEmptyString);

impl From<NonEmptyString> for RecipientName {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

impl RecipientName {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Die Gegenpartei einer Buchung — **roh** getragen: ihr Name (**optional**, aber das
/// wichtigste Kategorie-Signal) und, falls der Datensatz sie führt, ihre `CreditorId`.
///
/// **Warum der Name optional ist:** nicht jede Buchung hat eine Gegenpartei — eine
/// Entgelt-/Gebührenzeile (z. B. `ENTGELTABSCHLUSS`, Zweck „Entgeltabrechnung siehe
/// Anlage") bucht die Bank auf das eigene Konto **ohne Empfänger**. Ein Pflichtfeld würde
/// genau solche Zeilen dead-lettern und damit ein Kern-Analyseziel (Gebühren-/
/// Umbuchungskosten, CONTEXT.md) verfehlen. Fehlt der Name, bleibt er `None`.
///
/// Die Einlese-Stufe **deutet nicht**, ob Firma oder Privatperson. Diese
/// Unterscheidung ist laut CONTEXT.md bewusst **heuristisch und fail-safe**: nur eine
/// vorhandene Gläubiger-ID beweist sicher eine Firma; ihr Fehlen beweist *keine*
/// Person (es könnte eine per Überweisung zahlende Firma sein). Weil sie damit nicht
/// sicher validierbar ist und über das Pseudonymisieren entscheidet, gehört sie in
/// den nachgelagerten `obfuskieren`-Slice — `einlesen` reicht nur die validierten
/// Rohdaten weiter. Beide `Option`-Felder sind deshalb kein Zustands-Flag (ADR-0005),
/// sondern genuin optionale Datenfelder; der Summentyp Firma/Person entsteht erst
/// dort, wo die Deutung fällt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    name: Option<RecipientName>,
    creditor_id: Option<CreditorId>,
}

impl Recipient {
    /// Baut die Gegenpartei aus ihren — bereits nicht-leeren — Teilen. Beide sind genuin
    /// optional: ein fehlender Name (`None`) heißt eine Buchung ohne Gegenpartei (Gebühr),
    /// kein Fehler; eine fehlende Gläubiger-ID ebenso wenig.
    #[must_use]
    pub fn from_parts(name: Option<RecipientName>, creditor_id: Option<CreditorId>) -> Self {
        Self { name, creditor_id }
    }

    #[must_use]
    pub fn name(&self) -> Option<&RecipientName> {
        self.name.as_ref()
    }

    #[must_use]
    pub fn creditor_id(&self) -> Option<&CreditorId> {
        self.creditor_id.as_ref()
    }
}
