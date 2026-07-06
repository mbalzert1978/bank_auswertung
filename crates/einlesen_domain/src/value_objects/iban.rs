use super::NonEmptyString;

/// Eine IBAN, **roh** getragen — für Auftrags- wie Gegenkonto. In der Einlese-Stufe
/// nur auf „nicht leer" geprüft (das trägt bereits der `NonEmptyString`): die
/// **Prüfsummen-Validierung (mod 97)** ist bewusst **nicht** hier (sie deutet die IBAN
/// als Kontowert — Sache späterer Stufen), und es gibt bewusst **keinen
/// `Serialize`-Guard** (ADR-0008-Reframe: rohe Werte dürfen lokal durch die Outbox
/// reisen, kein Egress; die Domäne bindet ohnehin kein serde).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iban(NonEmptyString);

impl From<NonEmptyString> for Iban {
    /// Übernimmt den bereits nicht-leeren Wert roh — ob eine fehlende IBAN ein Fehler
    /// ist, entscheidet das Aggregat je Feld (Auftragskonto Pflicht, Gegenkonto optional).
    fn from(wert: NonEmptyString) -> Self {
        Self(wert)
    }
}

impl Iban {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
