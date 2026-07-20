use crate::value_objects::{NonEmptyString, Recipient};

/// Die **Zahlungsintermediäre** — Durchleiter, die im Kontoauszug als Gegenpartei erscheinen,
/// obwohl der eigentliche Händler dahinter steckt. Sie stehen hier als Marker im **normalisierten**
/// Firmennamen, denn ihre Firmierung trägt Beiwerk („PayPal Europe S.a.r.l. et Cie S.C.A.").
///
/// Das ist **Fachwissen der Domäne**, keine Konfiguration: dass ein Durchleiter den Händler
/// verdeckt, ist eine Aussage über die Analyse, nicht über die Umgebung.
const INTERMEDIARIES: [&str; 3] = ["PAYPAL", "KLARNA", "AMAZON PAYMENTS"];

/// Der **Fingerabdruck** — der stabile Schlüssel, der eine *Art* von Buchung identifiziert, nicht
/// die einzelne Buchung. Er ist **Domänenwissen** (ADR-0004, ADR-0013 Entscheidung F): entfernt man
/// das LLM, bleibt er gültig. Auf ihm ruhen die drei Nutzer, die noch kommen — die
/// Wiederkehr-Erkennung, die gelernten Korrekturen und die Deduplizierung der
/// Kategorisierer-Anfrage.
///
/// Er entsteht aus der Gegenpartei ([`Recipient`]) — und bei einem **Zahlungsintermediär**
/// zusätzlich aus dem Händler-Token des Verwendungszwecks, weil der Durchleiter sonst alle
/// dahinterliegenden Händler zu einem Klumpen zusammenfasste.
///
/// **Datenschutz:** der Fingerabdruck trägt nur, was schon stromabwärts der Obfuskierungs-Grenze
/// steht — die öffentliche Gläubiger-ID oder das seed-basierte Pseudonym, nie einen Rohwert und nie
/// den Seed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(String);

impl Fingerprint {
    /// Die **reine Funktion** `Gegenpartei → Fingerabdruck`. Kein Zustand, kein Port, kein I/O —
    /// dieselbe Eingabe ergibt immer dieselbe Ausgabe.
    ///
    /// Eine Firma wird über ihre **Gläubiger-ID** geschlüsselt (der öffentliche, über Importe
    /// hinweg stabile Firmenschlüssel); ist sie ein **Zahlungsintermediär**, tritt der
    /// Händler-Token des Verwendungszwecks hinzu. Eine Privatperson wird über ihr **seed-basiertes
    /// Pseudonym** geschlüsselt — derselbe Mensch ergibt bei gleichem Seed denselben Fingerabdruck,
    /// ohne dass preisgegeben wird, wer dahintersteckt (ADR-0010).
    ///
    /// Eine Buchung **ohne** Gegenpartei (Entgeltzeile) bekommt `None`: es gibt nichts, was ihre
    /// Art unterscheidbar machte, und ein Platzhalter-Schlüssel wäre ein Sammelbecken, das alle
    /// Entgeltzeilen zu einer Art verklumpte.
    #[must_use]
    pub(crate) fn of(recipient: &Recipient, purpose: Option<&NonEmptyString>) -> Option<Self> {
        match recipient {
            Recipient::Company { name, creditor_id } => {
                let company = format!("COMPANY:{}", creditor_id.as_str());
                // Der Händler-Token tritt **nur** hinzu, wenn die Firma ein Durchleiter ist *und*
                // ein verwertbarer Verwendungszweck vorliegt. Bei einer gewöhnlichen Firma bliebe
                // er weg — dort ist die Gläubiger-ID bereits der Händler, und der wechselnde
                // Zweck würde den Schlüssel je Buchung aufspalten.
                Some(Self(
                    match name
                        .as_ref()
                        .filter(|name| is_intermediary(name))
                        .and(purpose)
                        .and_then(merchant_token)
                    {
                        Some(token) => format!("{company}|MERCHANT:{token}"),
                        None => company,
                    },
                ))
            }
            Recipient::Person { pseudonym } => Some(Self(format!("PERSON:{}", pseudonym.as_str()))),
            Recipient::Absent => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Die alphanumerischen Wörter eines Rohtexts — der eine Zerleger, auf dem beide Normalisierungen
/// unten aufsetzen. Satzzeichen, Punkte und Mehrfach-Leerzeichen fallen dabei weg, damit kosmetische
/// Unterschiede denselben Händler nicht in zwei Arten spalten.
fn words(raw: &str) -> impl Iterator<Item = &str> {
    raw.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// Ist diese Firma ein **Durchleiter**? Geprüft wird auf dem normalisierten Namen, weil die
/// Firmierung Beiwerk trägt („PayPal Europe S.a.r.l. et Cie S.C.A." → `PAYPAL EUROPE S A R L …`).
fn is_intermediary(name: &NonEmptyString) -> bool {
    let normalised = words(name.as_str())
        .map(str::to_uppercase)
        .collect::<Vec<_>>()
        .join(" ");
    INTERMEDIARIES
        .iter()
        .any(|intermediary| normalised.contains(intermediary))
}

/// Der **Händler-Token** eines Verwendungszwecks: seine Wörter, normalisiert — **ohne** jedes Wort,
/// das eine Ziffer trägt.
///
/// Das Weglassen der Ziffern-Wörter ist der Kern: der Zweck eines Durchleiters führt eine **je
/// Buchung wechselnde** Referenznummer (`PP.4711.PP`). Ginge sie ein, bekäme jede einzelne Buchung
/// ihren eigenen Schlüssel — der Fingerabdruck identifizierte dann die Buchung statt ihrer Art, und
/// genau das ist er nicht.
///
/// **Heuristisch, nicht beweisbar vollständig** — sie trennt Händler zuverlässig, sie garantiert
/// nicht, dass jede Schreibweise desselben Händlers zusammenfällt.
fn merchant_token(purpose: &NonEmptyString) -> Option<String> {
    let token = words(purpose.as_str())
        .filter(|word| !word.chars().any(char::is_numeric))
        .map(str::to_uppercase)
        .collect::<Vec<_>>()
        .join("-");
    (!token.is_empty()).then_some(token)
}
