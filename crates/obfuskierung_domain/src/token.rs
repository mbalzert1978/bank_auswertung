use crate::non_empty_string::NonEmptyString;

/// Obfuskierte Kennung eines eigenen Kontos (`KONTO_x`) — der **Ausgang** der
/// Obfuskierung. Anders als die rohe IBAN ist dieses Token unbedenklich: es trägt
/// keinen strukturellen Identifikator, nur ein sprechendes Ersatz-Etikett.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KontoIdEntity(NonEmptyString);

impl KontoIdEntity {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    /// Das Token als Text (`KONTO_x`).
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Stabiles Ersatz-Token für eine Privatperson (`PERSON_x`). Die
/// Klartext-Zuordnung bleibt im separaten [`ZuordnungsSpeicher`](crate::ZuordnungsSpeicher)
/// und verlässt die Maschine nie ([ADR-0006](../../../docs/adr/0006-pseudonym-stabilitaet.md));
/// nach außen geht nur dieses Token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PseudonymEntity(NonEmptyString);

impl PseudonymEntity {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    /// Das Token als Text (`PERSON_x`).
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Freitext (Empfänger/Verwendungszweck) nach Obfuskierung. Minimale Erstversion:
/// trimmt nur (via [`NonEmptyString`]) und lehnt Leeres ab. Das eigentliche
/// Scrubbing eingebetteter Identifikatoren (IBAN-/BIC-Muster, bekannte
/// Personen-Token) vertieft ein späterer Schritt ([ADR-0003](../../../docs/adr/0003-rust-als-sprache.md));
/// dieser Typ erzwingt schon jetzt den einen Engpass, durch den aller Freitext
/// muss.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObfuskierterTextEntity(NonEmptyString);

impl ObfuskierterTextEntity {
    /// Liefert `None`, wenn der Wert nach dem Trimmen leer ist.
    pub fn new(value: impl AsRef<str>) -> Option<Self> {
        NonEmptyString::new(value).map(Self)
    }

    /// Der obfuskierte Text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
