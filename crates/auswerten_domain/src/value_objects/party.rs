use crate::value_objects::{CreditorId, NonEmptyString, Recipient};

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

/// Die **einmal eingeordnete** Gegenpartei (Issue 0011, W4) — die eine Stelle, an der feststeht, ob
/// auf der anderen Seite ein **Durchleiter**, eine gewöhnliche **Firma**, eine **Person** oder
/// **niemand** stand.
///
/// Auf ihr ruhen zwei vollständige `match`-Funktionen, und genau dafür gibt es sie: der
/// [`Fingerprint`](crate::Fingerprint) und der Anfrage-Text ans Orakel. Stünde die
/// Durchleiter-Erkennung in beiden einzeln, könnte der eine eine Buchung als Durchleiter sehen und der
/// andere nicht — dann ginge für PayPal der Firmenname hinaus, obwohl der Händler im Zweck steckt.
///
/// Eine geliehene Sicht auf die [`Counterparty`](crate::Counterparty), kein eigener Zustand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Party<'a> {
    /// Eine Firma, die Zahlungen nur **durchleitet** (PayPal, Klarna, Amazon): der eigentliche
    /// Händler steht im Verwendungszweck, nicht im Namen.
    Intermediary {
        creditor_id: &'a CreditorId,
        purpose: Option<&'a NonEmptyString>,
    },
    /// Eine gewöhnliche Firma — die Gläubiger-ID **ist** der Händler, ihr Name das Kategorie-Signal.
    Company {
        creditor_id: &'a CreditorId,
        name: Option<&'a NonEmptyString>,
    },
    /// Eine fail-safe Privatperson: nur ihr seed-basiertes Pseudonym.
    Person { pseudonym: &'a NonEmptyString },
    /// Keine Gegenpartei (Entgeltzeile).
    Absent,
}

impl<'a> Party<'a> {
    /// Ordnet die Gegenpartei ein — zuerst über die **Gläubiger-ID**, dann über den optionalen
    /// Namen (siehe [`is_intermediary`]).
    pub(crate) fn of(recipient: &'a Recipient, purpose: Option<&'a NonEmptyString>) -> Self {
        match recipient {
            Recipient::Company { name, creditor_id } => {
                match is_intermediary(name.as_ref(), creditor_id) {
                    true => Party::Intermediary {
                        creditor_id,
                        purpose,
                    },
                    false => Party::Company {
                        creditor_id,
                        name: name.as_ref(),
                    },
                }
            }
            Recipient::Person { pseudonym } => Party::Person { pseudonym },
            Recipient::Absent => Party::Absent,
        }
    }
}

/// Die alphanumerischen Wörter eines Rohtexts — Satzzeichen, Punkte und Mehrfach-Leerzeichen fallen
/// weg, damit kosmetische Unterschiede denselben Händler nicht in zwei Arten spalten.
pub(crate) fn words(raw: &str) -> impl Iterator<Item = &str> {
    raw.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// Ist diese Firma ein **Durchleiter**?
///
/// Zuerst über die **Gläubiger-ID** — das Pflichtfeld, das immer da ist. Erst danach über den
/// **optionalen** Namen, der als Auffangsignal dient, solange die ID oben fehlt. Die Reihenfolge ist
/// der Punkt: das verlässliche Signal muss zuerst greifen, sonst hinge die Händler-Trennung an der
/// Anwesenheit eines Feldes, das fehlen darf.
fn is_intermediary(name: Option<&NonEmptyString>, creditor_id: &CreditorId) -> bool {
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

/// Die **Egress-Wortregel** (Issue 0011, B1) — es gibt sie **genau einmal**: zerlegt an Leerraum,
/// fällt jedes Wort, das eine **Ziffer** oder ein **`@`** trägt.
///
/// Sie gilt für alles, was aus Freitext stammt und die Maschine verlassen kann: den Händler-Token im
/// [`Fingerprint`](crate::Fingerprint) und den Anfrage-Text ans Orakel. Ein Wort mit Ziffer ist eine
/// Referenz-, Kunden-, Vertrags- oder Telefonnummer, ein Datum oder der Rest einer verfremdeten IBAN;
/// ein Wort mit `@` eine E-Mail-Adresse. Für die Kategorie sagen sie nichts, über eine Person viel.
///
/// **Sie läuft vor jeder weiteren Zerlegung.** Zerlegte man erst an Satzzeichen, wäre aus
/// `max.mustermann@web.de` schon `MAX`, `MUSTERMANN`, `WEB`, `DE` geworden — vier unverdächtige
/// Wörter. Als ganzes Wort trägt die Adresse ihr `@` noch und fällt.
///
/// **Heuristisch, nicht beweisbar vollständig:** ein Name ohne Ziffer und ohne `@` bleibt stehen,
/// wenn ihn der Scrubber von `obfuskieren` nicht kennt.
pub(crate) fn egress_words(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
        .filter(|word| !word.chars().any(|c| c.is_numeric() || c == '@'))
}
