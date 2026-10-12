use crate::value_objects::non_empty_string::NonEmptyString;

/// Die **bekannten Personen-Namen**, die der Freitext-Engpass ([`ObfuscatedText`](crate::ObfuscatedText))
/// im Verwendungszweck ersetzt — ein reiner **Wert**, den die Domäne über den Port
/// [`KnownPersonsSource`](crate::KnownPersonsSource) bekommt. Woher die Namen stammen (bei `io`: die
/// Empfänger-Namen aus dem einlesen-Bestand plus eine lokale Liste), weiß die Domäne nicht; sie liest
/// weder Datei noch Datenbank.
///
/// **Keine Klartext↔Token-Zuordnung** ([ADR-0010](../../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)):
/// die Liste trägt nur Namen, kein Token und keinen Seed. Sie bleibt lokal und wird nie abgelegt.
///
/// **Wie ein Name gefunden wird** (heuristisch, nicht beweisbar vollständig):
/// - Groß-/Kleinschreibung spielt keine Rolle (`ERIKA BEISPIEL` trifft `Erika Beispiel`).
/// - Zwischen den Namensteilen darf beliebiger Leerraum oder ein Komma stehen.
/// - Ein mehrteiliger Name trifft auch in der Bank-Schreibweise „Nachname, Vorname".
/// - Der Name muss als **ganzes Wort** stehen — `Rose` trifft nicht `Rosenmontag`; ein angehängtes
///   Genitiv-`s` (`Lisas`) zählt noch zum Namen.
/// - Umschriften (`Müller`/`Mueller`), Spitznamen und Initialen erkennt sie **nicht**.
///
/// Die `Debug`-Ausgabe nennt nur die Anzahl, nie einen Namen.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct KnownPersons {
    /// Die Suchmuster, **längste zuerst** — so ersetzt „Lisa Beispiel" den ganzen Namen, bevor ein
    /// ebenfalls bekanntes „Lisa" nur den Vornamen fände.
    patterns: Vec<NamePattern>,
}

impl KnownPersons {
    /// Baut die Liste aus Namen. Doppelte (auch in anderer Schreibweise) fallen zusammen; ein Name
    /// ohne einen einzigen Namensteil (nur Kommas) wird übergangen.
    #[must_use]
    pub fn new(names: impl IntoIterator<Item = NonEmptyString>) -> Self {
        let mut patterns: Vec<NamePattern> = names
            .into_iter()
            .flat_map(|name| NamePattern::variants(name.as_str()))
            .collect();
        patterns.sort_by(|left, right| {
            right
                .char_count
                .cmp(&left.char_count)
                .then_with(|| left.key.cmp(&right.key))
        });
        patterns.dedup_by(|left, right| left.key == right.key);
        Self { patterns }
    }

    /// Wie viele Zeichen am Anfang von `text` ein bekannter Name belegt — `None`, wenn dort keiner
    /// steht. Dass `text` an einem Wortanfang beginnt, prüft der Aufrufer.
    pub(crate) fn match_length(&self, text: &[char]) -> Option<usize> {
        self.patterns
            .iter()
            .find_map(|pattern| pattern.match_length(text))
    }
}

impl std::fmt::Debug for KnownPersons {
    /// Redigiert: nur die Zahl der Suchmuster, nie ein Name — ein `{:?}` in einer Panik oder einem
    /// Log darf keinen Klarnamen tragen.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnownPersons")
            .field("patterns", &self.patterns.len())
            .finish()
    }
}

/// Ein Suchmuster: die Namensteile in Lese-Reihenfolge.
#[derive(Clone, PartialEq, Eq)]
struct NamePattern {
    tokens: Vec<Vec<char>>,
    /// Klein geschriebener Vergleichsschlüssel — fasst Schreibweisen zusammen.
    key: String,
    /// Summe der Zeichen aller Teile — die Sortierung „längste zuerst".
    char_count: usize,
}

impl NamePattern {
    /// Die Suchmuster eines Namens: der Name selbst und — hat er mehrere Teile — die Bank-Schreibweise
    /// mit dem letzten Teil vorn („Beispiel, Lisa").
    fn variants(name: &str) -> Vec<Self> {
        let tokens: Vec<Vec<char>> = name
            .split(|c: char| is_separator(c))
            .filter(|token| !token.is_empty())
            .map(|token| token.chars().collect())
            .collect();
        let rotated = tokens
            .split_last()
            .filter(|(_, rest)| !rest.is_empty())
            .map(|(last, rest)| std::iter::once(last).chain(rest).cloned().collect());
        std::iter::once(tokens)
            .chain(rotated)
            .filter(|tokens| !tokens.is_empty())
            .map(Self::from_tokens)
            .collect()
    }

    fn from_tokens(tokens: Vec<Vec<char>>) -> Self {
        let key = tokens
            .iter()
            .map(|token| {
                token
                    .iter()
                    .flat_map(|c| c.to_lowercase())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        let char_count = tokens.iter().map(Vec::len).sum();
        Self {
            tokens,
            key,
            char_count,
        }
    }

    /// Prüft das Muster am Anfang von `text`: erster Teil, dann je ein Trenner und der nächste Teil,
    /// zuletzt ein optionales Genitiv-`s` und ein Wortende.
    fn match_length(&self, text: &[char]) -> Option<usize> {
        let (first, others) = self.tokens.split_first()?;
        let after_first = strip_token(text, first)?;
        let after_name = others.iter().try_fold(after_first, |rest, token| {
            strip_token(strip_separator(rest)?, token)
        })?;
        let after_genitive = strip_genitive(after_name);
        ends_word(after_genitive).then(|| text.len() - after_genitive.len())
    }
}

/// Trennt Namensteile: Leerraum oder ein Komma.
fn is_separator(c: char) -> bool {
    c.is_whitespace() || c == ','
}

/// Der Rest hinter `token`, wenn `text` (ohne Rücksicht auf Groß-/Kleinschreibung) mit ihm beginnt.
fn strip_token<'t>(text: &'t [char], token: &[char]) -> Option<&'t [char]> {
    let (head, tail) = text.split_at_checked(token.len())?;
    head.iter()
        .zip(token)
        .all(|(left, right)| same_letter(*left, *right))
        .then_some(tail)
}

/// Der Rest hinter mindestens einem Trennzeichen — `None`, wenn keines folgt.
fn strip_separator(text: &[char]) -> Option<&[char]> {
    let skipped = text.iter().take_while(|c| is_separator(**c)).count();
    (skipped > 0).then(|| text.get(skipped..)).flatten()
}

/// Verschluckt ein angehängtes Genitiv-`s` (`Lisas`), wenn danach das Wort endet.
fn strip_genitive(text: &[char]) -> &[char] {
    match text.split_first() {
        Some(('s' | 'S', tail)) if ends_word(tail) => tail,
        _ => text,
    }
}

/// Ob an dieser Stelle ein Wort endet: Textende oder ein Zeichen, das kein Buchstabe/keine Ziffer ist.
fn ends_word(text: &[char]) -> bool {
    text.first().is_none_or(|c| !c.is_alphanumeric())
}

/// Gleicher Buchstabe ohne Rücksicht auf Groß-/Kleinschreibung.
fn same_letter(left: char, right: char) -> bool {
    left == right || left.to_lowercase().eq(right.to_lowercase())
}
