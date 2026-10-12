use crate::value_objects::party::{Party, egress_words, words};
use crate::value_objects::{Counterparty, NonEmptyString};

/// Der **Fingerabdruck** — der stabile Schlüssel, der eine *Art* von Buchung identifiziert, nicht
/// die einzelne Buchung. Er ist **Domänenwissen** (ADR-0004, ADR-0013 Entscheidung F): entfernt man
/// das LLM, bleibt er gültig. Auf ihm ruhen die drei Nutzer, die noch kommen — die
/// Wiederkehr-Erkennung, die gelernten Korrekturen und die Deduplizierung der
/// Kategorisierer-Anfrage.
///
/// Er entsteht aus der eingeordneten Gegenpartei (Durchleiter / Firma / Person / keine) — und bei einem **Zahlungsintermediär**
/// zusätzlich aus dem Händler-Token des Verwendungszwecks, weil der Durchleiter sonst alle
/// dahinterliegenden Händler zu einem Klumpen zusammenfasste.
///
/// **Datenschutz:** der Fingerabdruck trägt nur, was schon stromabwärts der Obfuskierungs-Grenze
/// steht — die öffentliche Gläubiger-ID oder das seed-basierte Pseudonym, nie einen Rohwert und nie
/// den Seed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(String);

impl Fingerprint {
    /// Die **reine Funktion** [`Counterparty`] → Fingerabdruck. Kein Zustand, kein Port, kein I/O —
    /// dieselbe Eingabe ergibt immer dieselbe Ausgabe. Ein vollständiges `match` auf die einmal
    /// eingeordnete Gegenpartei — dieselbe Einordnung, aus der auch der Anfrage-Text ans Orakel
    /// entsteht (Issue 0011).
    ///
    /// - Ein **Durchleiter** wird über seine Gläubiger-ID geschlüsselt, dazu der Händler-Token seines
    ///   Verwendungszwecks (sonst verklumpte er alle Händler dahinter zu einem).
    /// - Eine gewöhnliche **Firma** allein über ihre **Gläubiger-ID** — der öffentliche, über Importe
    ///   hinweg stabile Firmenschlüssel; der wechselnde Zweck würde ihn je Buchung aufspalten.
    /// - Eine **Privatperson** über ihr **seed-basiertes Pseudonym** — derselbe Mensch ergibt bei
    ///   gleichem Seed denselben Fingerabdruck, ohne dass preisgegeben wird, wer dahintersteckt
    ///   (ADR-0010).
    /// - Eine Buchung **ohne** Gegenpartei (Entgeltzeile) bekommt `None`: ein Platzhalter-Schlüssel
    ///   wäre ein Sammelbecken, das alle Entgeltzeilen zu einer Art verklumpte.
    #[must_use]
    pub(crate) fn of(counterparty: &Counterparty) -> Option<Self> {
        let company = |creditor_id: &NonEmptyString| format!("COMPANY:{}", creditor_id.as_str());
        match counterparty.party() {
            Party::Intermediary {
                creditor_id,
                purpose,
            } => Some(Self(match purpose.and_then(merchant_token) {
                Some(token) => format!("{}|MERCHANT:{token}", company(creditor_id)),
                None => company(creditor_id),
            })),
            Party::Company { creditor_id, .. } => Some(Self(company(creditor_id))),
            Party::Person { pseudonym } => Some(Self(format!("PERSON:{}", pseudonym.as_str()))),
            Party::Absent => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Der **Händler-Token** eines Verwendungszwecks: erst die **Egress-Wortregel** (jedes Wort mit
/// Ziffer oder `@` fällt, als Ganzes), dann die übrigen Wörter an Satzzeichen zerlegt, großgeschrieben,
/// mit `-` verbunden.
///
/// Das Weglassen der Ziffern-Wörter ist der Kern: der Zweck eines Durchleiters führt eine **je
/// Buchung wechselnde** Referenznummer (`PP.4711.PP`). Ginge sie ein, bekäme jede einzelne Buchung
/// ihren eigenen Schlüssel — der Fingerabdruck identifizierte dann die Buchung statt ihrer Art. Und
/// weil der Token mit dem Fingerabdruck in den Chat-Export geht, fällt eine E-Mail-Adresse **vor** dem
/// Zerlegen, nicht als `MAX-MUSTERMANN-WEB-DE` danach (Issue 0011).
///
/// **Heuristisch, nicht beweisbar vollständig** — sie trennt Händler zuverlässig, sie garantiert
/// nicht, dass jede Schreibweise desselben Händlers zusammenfällt. Und ein **unbekannter Name** im
/// Zweck (einer, den der Scrubber von `obfuskieren` nicht kennt) bleibt im Token stehen.
fn merchant_token(purpose: &NonEmptyString) -> Option<String> {
    let token = egress_words(purpose.as_str())
        .flat_map(words)
        .map(str::to_uppercase)
        .collect::<Vec<_>>()
        .join("-");
    (!token.is_empty()).then_some(token)
}
