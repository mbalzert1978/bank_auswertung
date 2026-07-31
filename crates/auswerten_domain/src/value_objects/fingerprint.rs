use crate::value_objects::{Counterparty, NonEmptyString, Recipient};

/// Die **Gläubiger-IDs** bekannter Durchleiter — das **verlässliche** Erkennungssignal, denn die
/// Gläubiger-ID ist an einer Firma **Pflichtfeld** (der Adapter erzwingt ihre Präsenz), während der
/// Firmenname optional ist.
///
/// Hinge die Erkennung allein am Namen, fiele sie bei einer namenlosen Zeile **still** aus und der
/// Durchleiter verklumpte seine Händler wieder — ein Fehler, der nichts fehlschlagen ließe, sondern
/// nur ein falsches Ergebnis erzeugte.
///
/// **Herkunft (Issue 0047, HITL):** alle IDs stammen aus echten Auszugsdaten — vom Nutzer am
/// 20.07.2026 aus den Detailansichten eigener Lastschrift-Buchungen abgelesen. Die PayPal-ID war in
/// 0041 aus allgemeiner Kenntnis gesetzt und ist gegen dieselbe Quelle **verifiziert**. Dass auch
/// Amazons Direkt-Firmierungen (Digital/EU) hier stehen, ist eine bewusste Analyse-Entscheidung:
/// ihr Verwendungszweck trägt die *Art* (Prime vs. Kindle vs. Einkauf), die Firmierung nur den
/// Kanal — ohne Aufspaltung verklumpte jede zu einem Sammelbecken.
///
/// **Rest-Lücke:** Klarna hat weiterhin keine ID — der Dienst wird nicht genutzt, echte Quelldaten
/// existieren nicht, und eine aus allgemeiner Kenntnis gesetzte ID wäre schlimmer als keine (sie
/// behandelte still die falsche Firma als Durchleiter). Bis echte Daten auftauchen, trägt Klarna
/// allein der Namens-Marker unten.
const INTERMEDIARY_CREDITOR_IDS: [&str; 4] = [
    // PayPal Europe S.a.r.l. et Cie S.C.A.
    "LU96ZZZ0000000000000000058",
    // Amazon Payments Europe S.C.A. — der Einzieher der Marketplace-Käufe.
    "DE94ZZZ00000561653",
    // Amazon Digital Germany GmbH — Prime/Kindle & Co.
    "DE96ZZZ00000594888",
    // Amazon EU S.A R.L., Niederlassung Deutschland — der Direkteinkauf.
    "DE24ZZZ00000561652",
];

/// Namens-Marker der Durchleiter — das **nachgelagerte Auffangsignal** hinter der Gläubiger-ID:
/// es trägt allein **Klarna** (dessen ID oben noch fehlt) und fängt sonst nur eine Firmierung auf,
/// deren ID noch nicht hinterlegt ist. Die Marker stehen im **normalisierten** Namen, denn die
/// Firmierung trägt Beiwerk („PayPal Europe S.a.r.l. et Cie S.C.A.").
///
/// Beides ist **Fachwissen der Domäne**, keine Konfiguration: dass ein Durchleiter den Händler
/// verdeckt, ist eine Aussage über die Analyse, nicht über die Umgebung.
const INTERMEDIARY_NAMES: [&str; 3] = ["PAYPAL", "KLARNA", "AMAZON PAYMENTS"];

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
    /// Die **reine Funktion** [`Counterparty`] → Fingerabdruck. Kein Zustand, kein Port, kein I/O —
    /// dieselbe Eingabe ergibt immer dieselbe Ausgabe.
    ///
    /// Eine Firma wird über ihre **Gläubiger-ID** geschlüsselt (der öffentliche, über Importe
    /// hinweg stabile Firmenschlüssel); ist sie ein **Zahlungsintermediär**, tritt der
    /// Händler-Token des Verwendungszwecks hinzu — erkannt vorrangig an eben dieser Gläubiger-ID,
    /// nicht am optionalen Namen (siehe [`is_intermediary`]). Eine Privatperson wird über ihr **seed-basiertes
    /// Pseudonym** geschlüsselt — derselbe Mensch ergibt bei gleichem Seed denselben Fingerabdruck,
    /// ohne dass preisgegeben wird, wer dahintersteckt (ADR-0010).
    ///
    /// Eine Buchung **ohne** Gegenpartei (Entgeltzeile) bekommt `None`: es gibt nichts, was ihre
    /// Art unterscheidbar machte, und ein Platzhalter-Schlüssel wäre ein Sammelbecken, das alle
    /// Entgeltzeilen zu einer Art verklumpte.
    #[must_use]
    pub(crate) fn of(counterparty: &Counterparty) -> Option<Self> {
        let purpose = counterparty.purpose();
        match counterparty.recipient() {
            Recipient::Company { name, creditor_id } => {
                let company = format!("COMPANY:{}", creditor_id.as_str());
                // Der Händler-Token tritt **nur** hinzu, wenn die Firma ein Durchleiter ist *und*
                // ein verwertbarer Verwendungszweck vorliegt. Bei einer gewöhnlichen Firma bliebe
                // er weg — dort ist die Gläubiger-ID bereits der Händler, und der wechselnde
                // Zweck würde den Schlüssel je Buchung aufspalten.
                Some(Self(
                    match is_intermediary(name.as_ref(), creditor_id)
                        .then_some(purpose)
                        .flatten()
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

/// Ist diese Firma ein **Durchleiter**?
///
/// Zuerst über die **Gläubiger-ID** — das Pflichtfeld, das immer da ist. Erst danach über den
/// **optionalen** Namen, der als Auffangsignal dient, solange die ID oben fehlt. Die Reihenfolge ist
/// der Punkt: das verlässliche Signal muss zuerst greifen, sonst hinge die Händler-Trennung an der
/// Anwesenheit eines Feldes, das fehlen darf.
fn is_intermediary(name: Option<&NonEmptyString>, creditor_id: &NonEmptyString) -> bool {
    let normalised_id = words(creditor_id.as_str())
        .map(str::to_uppercase)
        .collect::<String>();
    INTERMEDIARY_CREDITOR_IDS.contains(&normalised_id.as_str())
        || name.is_some_and(|name| {
            let normalised = words(name.as_str())
                .map(str::to_uppercase)
                .collect::<Vec<_>>()
                .join(" ");
            INTERMEDIARY_NAMES
                .iter()
                .any(|intermediary| normalised.contains(intermediary))
        })
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
