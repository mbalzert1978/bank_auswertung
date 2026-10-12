//! Die **Scrubbing-Regeln** des Freitext-Engpasses ([`ObfuscatedText`](crate::ObfuscatedText),
//! [Issue 0010](../../../docs/issues/0010-scrubbing-regeln-fuer-obfuskiertertext.md)): IBAN-Muster,
//! BIC-Muster und bekannte Personen-Namen werden durch einen **festen Platzhalter** ersetzt.
//!
//! **Heuristisch, nicht beweisbar vollständig.** Die Regeln fangen die üblichen Schreibweisen; eine
//! verfremdete IBAN, ein unbekannter Name oder eine ausländische BIC ohne Hinweis rutschen durch.
//! Wo die Regeln unsicher sind, ersetzen sie lieber zu viel als zu wenig (fail-safe).
//!
//! **Warum ein fester Platzhalter und kein `PERSON_x`-Token:** ADR-0010 hält fest, dass Freitext
//! **nicht** seed-tokenisiert wird. Ein fester Platzhalter verrät nichts — auch nicht, dass zwei
//! Verwendungszwecke dieselbe Person nennen —, und er braucht weder Seed noch Obfuskator.
//!
//! Ohne externes Crate (die Domäne ist dependency-frei): die Muster sind von Hand geschrieben.

use crate::value_objects::known_persons::KnownPersons;

/// Kürzeste IBAN (Norwegen) in kompakter Schreibweise.
const IBAN_MIN_LENGTH: usize = 15;
/// Längste IBAN in kompakter Schreibweise.
const IBAN_MAX_LENGTH: usize = 34;

/// Wörter, nach denen ein BIC-förmiges Wort sicher eine BIC ist (`BIC: …`, `SWIFT …`).
const BIC_MARKERS: [&str; 2] = ["BIC", "SWIFT"];

/// Welche Art Identifikator an einer Stelle erkannt wurde — bestimmt den Platzhalter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Identifier {
    Iban,
    Bic,
    Person,
}

impl Identifier {
    fn placeholder(self) -> &'static str {
        match self {
            Identifier::Iban => "[IBAN]",
            Identifier::Bic => "[BIC]",
            Identifier::Person => "[PERSON]",
        }
    }
}

/// Ersetzt im Text jede erkannte IBAN, BIC und jeden bekannten Personen-Namen durch seinen
/// Platzhalter. Ein einziger Durchlauf von links nach rechts: ein eingesetzter Platzhalter wird nicht
/// erneut geprüft.
///
/// Der Lauf ist eine Zustandsmaschine über die Zeichen (Position + Ausgabe) — das ist hier lesbarer
/// als eine Iterator-Kette, weil ein Treffer mehrere Zeichen auf einmal verbraucht.
pub(crate) fn scrub(text: &str, persons: &KnownPersons) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut scrubbed = String::with_capacity(text.len());
    let mut position = 0;
    while let Some((before, rest)) = chars.split_at_checked(position) {
        let Some(current) = rest.first() else {
            break;
        };
        if let Some((identifier, length)) = identifier_at(before, rest, persons) {
            scrubbed.push_str(identifier.placeholder());
            position += length;
        } else {
            scrubbed.push(*current);
            position += 1;
        }
    }
    scrubbed
}

/// Der Identifikator, der am Anfang von `rest` steht, samt seiner Länge in Zeichen. `before` ist der
/// Text davor — er entscheidet, ob hier ein Wort beginnt und ob ein BIC-Hinweis vorausgeht.
///
/// Eine IBAN wird **auch mitten im Wort** gesucht (`IBANDE00…` ist zusammengeklebt, aber eine IBAN);
/// Namen und BICs nur am Wortanfang — sonst fiele jedes Wort, das zufällig so beginnt.
fn identifier_at(
    before: &[char],
    rest: &[char],
    persons: &KnownPersons,
) -> Option<(Identifier, usize)> {
    iban_length(rest)
        .map(|length| (Identifier::Iban, length))
        .or_else(|| {
            starts_word(before)
                .then(|| {
                    persons
                        .match_length(rest)
                        .map(|length| (Identifier::Person, length))
                        .or_else(|| {
                            bic_length(before, rest).map(|length| (Identifier::Bic, length))
                        })
                })
                .flatten()
        })
}

/// **IBAN-Muster:** zwei Buchstaben (Land), zwei Ziffern (Prüfziffer), dann Buchstaben/Ziffern — am
/// Stück oder in Gruppen mit je einem Leerzeichen (`DE00 1234 5678 …`). Trifft, sobald mindestens
/// 15 Zeichen zusammenkommen.
///
/// Die Prüfziffer wird **nicht** nachgerechnet: auch eine vertippte IBAN ist eine IBAN (fail-safe).
/// Eine Gruppe nach einem Leerzeichen zählt noch dazu, wenn sie eine Ziffer trägt oder aus vier
/// Großbuchstaben besteht (`GB00 FIKT …`); so endet die IBAN vor dem nächsten gewöhnlichen Wort,
/// verschluckt im Zweifel aber eher eines zu viel. Mehr als 34 Zeichen sammelt sie über Gruppen nicht.
fn iban_length(rest: &[char]) -> Option<usize> {
    let head = ascii_alphanumeric_run(rest);
    let start = rest.get(..head)?;
    let [country_1, country_2, check_1, check_2, ..] = start else {
        return None;
    };
    let iban_start = country_1.is_ascii_alphabetic()
        && country_2.is_ascii_alphabetic()
        && check_1.is_ascii_digit()
        && check_2.is_ascii_digit();
    if !iban_start {
        return None;
    }
    let (consumed, compact) = iban_groups(rest, head);
    (compact >= IBAN_MIN_LENGTH).then_some(consumed)
}

/// Sammelt die durch je ein Leerzeichen abgetrennten Gruppen hinter dem IBAN-Anfang. Gibt die
/// verbrauchten Zeichen (mit Leerzeichen) und die kompakte Länge (ohne) zurück.
fn iban_groups(rest: &[char], head: usize) -> (usize, usize) {
    let mut consumed = head;
    let mut compact = head;
    while let Some(group) = rest
        .get(consumed..)
        .and_then(|after| after.strip_prefix(&[' ']))
    {
        let length = ascii_alphanumeric_run(group);
        let accepted = group.get(..length).is_some_and(|candidate| {
            length > 0 && compact + length <= IBAN_MAX_LENGTH && iban_group(candidate)
        });
        if !accepted {
            break;
        }
        consumed += 1 + length;
        compact += length;
    }
    (consumed, compact)
}

/// Eine Folgegruppe einer IBAN: trägt eine Ziffer oder besteht aus genau vier Großbuchstaben.
fn iban_group(group: &[char]) -> bool {
    group.iter().any(char::is_ascii_digit)
        || (group.len() == 4 && group.iter().all(char::is_ascii_uppercase))
}

/// **BIC-Muster:** ein ganzes Wort aus 8 oder 11 Zeichen — sechs Großbuchstaben (Bank + Land), dann
/// Großbuchstaben/Ziffern (Ort, Filiale).
///
/// Die Form allein träfe auch deutsche Wörter in Großbuchstaben (`LASTSCHRIFT`, `PRIVATKONTO`,
/// `KREDITKARTE` haben genau diese Form). Darum zählt ein Wort nur als BIC, wenn zusätzlich **eines**
/// gilt:
/// - Ort oder Filiale tragen eine Ziffer (`FIKTDEF1`) — deutsche Wörter tragen keine;
/// - das Land ist `DE` (`FIKTDEFF`) — die BICs auf einem deutschen Konto sind fast alle deutsch, und
///   ein deutsches Wort mit `DE` an Stelle 5–6 und genau 8 oder 11 Buchstaben ist selten;
/// - ein Hinweis wie `BIC` oder `SWIFT` steht davor (`BIC: FIKTNLAA`).
///
/// Bekannte Lücke: eine ausländische BIC ohne Ziffer und ohne Hinweis bleibt stehen. Das ist der
/// bewusste Preis dafür, dass Buchungsarten wie `LASTSCHRIFT` lesbar bleiben — an ihnen hängt die
/// Gebühren-Klassifikation in `auswerten`.
fn bic_length(before: &[char], rest: &[char]) -> Option<usize> {
    let length = ascii_alphanumeric_run(rest);
    let candidate = rest.get(..length)?;
    let followed_by_letter = rest.get(length).is_some_and(|c| c.is_alphanumeric());
    let (bank_and_country, location) = candidate.split_at_checked(6)?;
    let country = bank_and_country.get(4..)?;
    let shaped = matches!(length, 8 | 11)
        && !followed_by_letter
        && bank_and_country.iter().all(char::is_ascii_uppercase)
        && location
            .iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let plausible = location.iter().any(char::is_ascii_digit)
        || country == ['D', 'E']
        || follows_bic_marker(before);
    (shaped && plausible).then_some(length)
}

/// Ob direkt vor der Stelle (über Leerraum und Doppelpunkt hinweg) ein BIC-Hinweis als ganzes Wort
/// steht.
fn follows_bic_marker(before: &[char]) -> bool {
    let end = before
        .iter()
        .rposition(|c| !(c.is_whitespace() || *c == ':'))
        .map_or(0, |index| index + 1);
    let Some(trimmed) = before.get(..end) else {
        return false;
    };
    BIC_MARKERS
        .iter()
        .any(|marker| ends_with_word(trimmed, marker))
}

/// Ob `text` mit `word` als ganzem Wort endet (ohne Rücksicht auf Groß-/Kleinschreibung).
fn ends_with_word(text: &[char], word: &str) -> bool {
    let word: Vec<char> = word.chars().collect();
    text.len()
        .checked_sub(word.len())
        .and_then(|split| text.split_at_checked(split))
        .is_some_and(|(head, tail)| {
            starts_word(head)
                && tail
                    .iter()
                    .zip(&word)
                    .all(|(left, right)| left.eq_ignore_ascii_case(right))
        })
}

/// Ob nach `before` ein Wort beginnt: Textanfang oder ein Zeichen davor, das kein Buchstabe/keine
/// Ziffer ist.
fn starts_word(before: &[char]) -> bool {
    before.last().is_none_or(|c| !c.is_alphanumeric())
}

/// Wie viele ASCII-Buchstaben/-Ziffern am Anfang stehen.
fn ascii_alphanumeric_run(text: &[char]) -> usize {
    text.iter()
        .take_while(|c| c.is_ascii_alphanumeric())
        .count()
}
