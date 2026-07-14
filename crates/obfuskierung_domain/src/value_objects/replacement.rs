use crate::value_objects::non_empty_string::NonEmptyString;

/// Der rohe Ersatzwert, den der [`Obfuscator`](crate::Obfuscator)-Port aus (Seed, Klartext)
/// erzeugt — die pseudo-zufällige, aber deterministische Kennung, aus der die Domäne dann ihre
/// sprechenden Token (`KONTO_…`/`PERSON_…`) formt. Trägt den rohen Klartext **nicht** (das ist
/// der Sinn der Obfuskierung); vom Port als vertrauenswürdig nicht-leer zugesichert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Replacement(String);

impl From<String> for Replacement {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Replacement {
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Obfuskierte Kennung eines Kontos (`KONTO_<ersatz>`) — der **Ausgang** der Obfuskierung für
/// eine IBAN (Auftraggeber **oder** Gegenkonto). Anders als die rohe IBAN unbedenklich: nur ein
/// sprechendes Präfix über dem seed-abgeleiteten Ersatzwert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountToken(String);

impl AccountToken {
    /// Formt das Konto-Token aus dem seed-abgeleiteten Ersatzwert (Präfix `KONTO_`).
    #[must_use]
    pub fn from_replacement(replacement: Replacement) -> Self {
        Self(format!("KONTO_{}", replacement.as_str()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stabiles Ersatz-Token für eine Privatperson (`PERSON_<ersatz>`). Die Klartext-Zuordnung
/// verlässt die Maschine nie ([ADR-0006](../../../docs/adr/0006-pseudonym-stabilitaet.md)); nach
/// außen geht nur dieses Token. Reproduzierbar aus Seed + Klartext-Name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pseudonym(String);

impl Pseudonym {
    /// Formt das Personen-Token aus dem seed-abgeleiteten Ersatzwert (Präfix `PERSON_`).
    #[must_use]
    pub fn from_replacement(replacement: Replacement) -> Self {
        Self(format!("PERSON_{}", replacement.as_str()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Freitext (Firmenname / Verwendungszweck / Buchungstext) nach Obfuskierung. Minimale
/// Erstversion: trimmt nur (via [`NonEmptyString`]) und lehnt Leeres ab. Das eigentliche
/// Scrubbing eingebetteter Identifikatoren (IBAN-/BIC-Muster, bekannte Personen-Token) vertieft
/// ein späterer Schritt; dieser Typ erzwingt schon jetzt den einen Engpass, durch den aller
/// Freitext muss.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObfuscatedText(NonEmptyString);

impl From<NonEmptyString> for ObfuscatedText {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

impl ObfuscatedText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Die **Gläubiger-Identifikation** (SEPA Creditor ID) einer Firma — der eine strukturelle
/// Identifikator, der die Maschine verlassen darf: eine öffentliche Firmenkennung, bewusst als
/// Analyse-Anker bewahrt (CONTEXT.md, [ADR-0001](../../../docs/adr/0001-hexagonale-architektur.md)).
/// Wird **nicht** obfuskiert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
