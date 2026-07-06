use super::NonEmptyString;
use crate::fehler::ZeilenError;

/// Die **Gläubiger-Identifikation** (SEPA Creditor ID) einer Firma — der eine
/// strukturelle Identifikator, der die Maschine verlassen darf: eine öffentliche
/// Firmenkennung, bewusst als Analyse-Anker der Abo-Erkennung bewahrt (CONTEXT.md,
/// ADR-0001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GläubigerId(NonEmptyString);

impl From<NonEmptyString> for GläubigerId {
    fn from(wert: NonEmptyString) -> Self {
        Self(wert)
    }
}

impl GläubigerId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Der (Klartext-)Name des Empfängers — das wichtigste Kategorie-Signal (CONTEXT.md).
/// In der Einlese-Stufe bewusst **roh** getragen; das Pseudonymisieren einer
/// Privatperson ist Sache des nachgelagerten `obfuskieren`-Slice, nicht dieser Stufe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empfängername(NonEmptyString);

impl From<NonEmptyString> for Empfängername {
    fn from(wert: NonEmptyString) -> Self {
        Self(wert)
    }
}

impl Empfängername {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Die Gegenpartei einer Buchung — **roh** getragen: ihr Name (Pflicht, wichtigstes
/// Kategorie-Signal) und, falls der Datensatz sie führt, ihre `GläubigerId`.
///
/// Die Einlese-Stufe **deutet nicht**, ob Firma oder Privatperson. Diese
/// Unterscheidung ist laut CONTEXT.md bewusst **heuristisch und fail-safe**: nur eine
/// vorhandene Gläubiger-ID beweist sicher eine Firma; ihr Fehlen beweist *keine*
/// Person (es könnte eine per Überweisung zahlende Firma sein). Weil sie damit nicht
/// sicher validierbar ist und über das Pseudonymisieren entscheidet, gehört sie in
/// den nachgelagerten `obfuskieren`-Slice — `einlesen` reicht nur die validierten
/// Rohdaten weiter. Das `Option`-Feld ist deshalb kein Zustands-Flag (ADR-0005),
/// sondern ein genuin optionales Datenfeld; der Summentyp Firma/Person entsteht erst
/// dort, wo die Deutung fällt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empfänger {
    name: Empfängername,
    gläubiger_id: Option<GläubigerId>,
}

impl Empfänger {
    /// Baut die Gegenpartei aus ihren — bereits nicht-leeren — Teilen. Der Name ist
    /// Pflicht: sein Fehlen (`None`) heißt keine identifizierbare Gegenpartei
    /// (`EmpfängerFehlt`, terminal). Eine fehlende Gläubiger-ID ist kein Fehler.
    pub fn aus_teilen(
        name: Option<Empfängername>,
        gläubiger_id: Option<GläubigerId>,
    ) -> Result<Self, ZeilenError> {
        let name = name.ok_or(ZeilenError::EmpfängerFehlt)?;
        Ok(Self {
            name, gläubiger_id
        })
    }

    pub fn name(&self) -> &Empfängername {
        &self.name
    }

    pub fn gläubiger_id(&self) -> Option<&GläubigerId> {
        self.gläubiger_id.as_ref()
    }
}
