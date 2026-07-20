use crate::value_objects::{Fingerprint, NonEmptyString};

/// Die **ausgehende** Anfrage ans Kategorisierer-Orakel — das schmale DTO der Richtung „hinaus"
/// (ADR-0002: **je Richtung ein eigenes** DTO, nie ein gemeinsames God-DTO für Frage und Antwort).
///
/// Sie trägt **zwei** Dinge und nichts sonst: den [`Fingerprint`] als **Schlüssel**, unter dem die
/// Antwort zurückfindet, und den obfuskierten **Text**, an dem das LLM die Kategorie erraten soll
/// (CONTEXT.md „Kategorie": Empfänger + Verwendungszweck).
///
/// **Warum der Schlüssel und nicht der Text die Zuordnung trägt:** die Antwort schlüsselt auf den
/// Fingerabdruck. Damit ist Re-Identifikation **strukturell** unmöglich statt bloß per Review
/// verhindert — es gibt keinen Rückweg vom Etikett zu einer Person, denn der Schlüssel ist selbst
/// schon ein Derivat (Gläubiger-ID oder seed-basiertes Pseudonym).
///
/// **Datenschutz:** dies ist das **Egress-DTO** dieses Slice — hier verlassen Daten die Maschine.
/// Ein struktureller Identifikator (IBAN/Kontonummer/BIC) kann in keinem der beiden Felder stehen:
/// der Fingerabdruck ist ein Derivat, und der Text hat stromaufwärts den
/// `ObfuskierterText`-Engpass passiert. Die compilerfeste „kein `Serialize`"-Sperre liegt auf der
/// Projektion dieses Werts an der Adapter-Grenze.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CategoryRequest {
    fingerprint: Fingerprint,
    text: NonEmptyString,
}

impl CategoryRequest {
    #[must_use]
    pub(crate) fn new(fingerprint: Fingerprint, text: NonEmptyString) -> Self {
        Self { fingerprint, text }
    }

    #[must_use]
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub fn text(&self) -> &NonEmptyString {
        &self.text
    }
}
