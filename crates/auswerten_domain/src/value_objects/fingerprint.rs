use crate::value_objects::party::{Party, egress_words, words};
use crate::value_objects::{Counterparty, CreditorId, NonEmptyString};

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
///
/// **Schlüssel und Anker.** Der [`as_str`](Self::as_str)-Schlüssel ist die stabile Zeichenkette, unter
/// der Register, Export und Wiederkehr die Art wiederfinden. Daneben trägt der Fingerabdruck seinen
/// [`KindAnchor`] — die **Struktur**, aus der der Schlüssel entstand. Wer die Art beschriften will
/// (der Bericht, Issue 0012), liest den Anker, statt den Schlüssel zu zerlegen. Beides entsteht in
/// `of` aus derselben Einordnung; der Schlüssel steht vorn und ordnet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint {
    key: String,
    anchor: KindAnchor,
}

/// Der **Anker** einer Buchungsart — wer auf der anderen Seite stand, als Struktur statt als Text.
/// Eine Buchung ohne Gegenpartei hat keinen (und keinen Fingerabdruck).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KindAnchor {
    /// Eine **Firma** über ihre öffentliche Gläubiger-ID. Bei einem **Durchleiter** steht der
    /// Händler-Token seines Verwendungszwecks daneben (Egress-Wortregel angewandt) — fehlt er, ist die
    /// Art der Durchleiter selbst.
    Company {
        creditor_id: CreditorId,
        merchant: Option<NonEmptyString>,
    },
    /// Eine fail-safe **Privatperson** über ihr seed-basiertes Pseudonym.
    Person { pseudonym: NonEmptyString },
}

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
        let anchor = match counterparty.party() {
            Party::Intermediary {
                creditor_id,
                purpose,
            } => KindAnchor::Company {
                creditor_id: creditor_id.clone(),
                merchant: purpose.and_then(merchant_token),
            },
            Party::Company { creditor_id, .. } => KindAnchor::Company {
                creditor_id: creditor_id.clone(),
                merchant: None,
            },
            Party::Person { pseudonym } => KindAnchor::Person {
                pseudonym: pseudonym.clone(),
            },
            Party::Absent => return None,
        };
        Some(Self {
            key: key_of(&anchor),
            anchor,
        })
    }

    /// Der stabile Schlüssel der Art.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.key
    }

    /// Wer auf der anderen Seite stand — als Struktur (siehe Typ-Doku).
    #[must_use]
    pub fn anchor(&self) -> &KindAnchor {
        &self.anchor
    }
}

/// Der Schlüssel eines Ankers — die **eine** Stelle, an der seine Zeichenketten-Form entsteht:
/// `COMPANY:<Gläubiger-ID>[|MERCHANT:<Händler-Token>]` bzw. `PERSON:<Pseudonym>`.
fn key_of(anchor: &KindAnchor) -> String {
    match anchor {
        KindAnchor::Company {
            creditor_id,
            merchant: Some(merchant),
        } => format!(
            "COMPANY:{}|MERCHANT:{}",
            creditor_id.as_str(),
            merchant.as_str()
        ),
        KindAnchor::Company {
            creditor_id,
            merchant: None,
        } => format!("COMPANY:{}", creditor_id.as_str()),
        KindAnchor::Person { pseudonym } => format!("PERSON:{}", pseudonym.as_str()),
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
fn merchant_token(purpose: &NonEmptyString) -> Option<NonEmptyString> {
    NonEmptyString::new(
        &egress_words(purpose.as_str())
            .flat_map(words)
            .map(str::to_uppercase)
            .collect::<Vec<_>>()
            .join("-"),
    )
}
