use crate::value_objects::known_persons::KnownPersons;
use crate::value_objects::non_empty_string::NonEmptyString;
use crate::value_objects::scrubbing;

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

/// Freitext (Firmenname / Verwendungszweck / Buchungstext) nach Obfuskierung — der **eine**
/// Engpass, durch den aller Freitext muss. Es gibt keinen Weg, ihn **ohne** Scrubbing zu bauen: der
/// einzige Konstruktor ist das crate-interne `scrub`, das die Gegenpartei-Obfuskierung aufruft.
/// Gescrubbt wird **heuristisch, nicht beweisbar vollständig**: IBAN-Muster, BIC-Muster und die
/// bekannten Personen-Namen ([`KnownPersons`]) werden durch feste Platzhalter ersetzt; was die
/// Regeln nicht kennen, bleibt stehen.
///
/// Nicht-leer **im Typ**: der Eingang ist ein getrimmter [`NonEmptyString`], und jeder ersetzte
/// Abschnitt wird zu einem nicht-leeren Platzhalter — das Ergebnis bleibt ein `NonEmptyString`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObfuscatedText(NonEmptyString);

impl ObfuscatedText {
    /// Scrubbt den Freitext **best-effort**, bevor er als `ObfuscatedText` existiert: eingebettete
    /// **IBANs** werden zu `[IBAN]`, **BICs** zu `[BIC]` und **bekannte Personen-Namen** zu
    /// `[PERSON]`.
    ///
    /// **Heuristisch, nicht beweisbar vollständig.** Die Regeln fangen die üblichen Schreibweisen;
    /// was sie nicht kennen, bleibt stehen — ein Name, der weder in [`KnownPersons`] steht noch
    /// Empfänger war, eine verfremdete IBAN, eine ausländische BIC ohne Ziffer und ohne `BIC`-Hinweis.
    /// Wo sie unsicher sind, ersetzen sie lieber zu viel (fail-safe): auch eine Firma ohne
    /// Gläubiger-ID, deren Name in der Liste steht, wird geschwärzt. Eine Anonymisierung ist das
    /// nicht. Die einzelnen Regeln und ihre Grenzen stehen am Modul `scrubbing`.
    #[must_use]
    pub(crate) fn scrub(raw: &NonEmptyString, persons: &KnownPersons) -> Self {
        Self(NonEmptyString::from_scrubbed(scrubbing::scrub(
            raw.as_str(),
            persons,
        )))
    }

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
