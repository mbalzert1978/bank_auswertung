//! Die **eine** Leak-Prüfung der Lauf-Tests: was aus den Eingaben eines Laufs stammt und nie in einer
//! Ausgabe (Bericht, Chat-Export) stehen darf — gelesen aus den Eingabe-Dateien selbst, damit die
//! Prüfung mit jeder Fixture mitwächst.
//!
//! Drei Klassen, je nach Art des Werts verschieden streng verglichen:
//!
//! - **ganze Werte** — jede IBAN (Auftragskonto, Gegenkonto, Eigene-Konten-Config, dort auch in
//!   Leerzeichen-Schreibweise), jede BIC, Mandats- und End-to-End-Referenz, jeder Name einer
//!   Gegenpartei **ohne** Gläubiger-ID (einer fail-safe Person), jeder Name der Namensliste und der
//!   Seed. Geprüft roh **und** normalisiert (großgeschrieben, nur Buchstaben und Ziffern), sodass auch
//!   eine IBAN ohne Leerzeichen oder mit Punkten auffällt.
//! - **Namenswörter** — jedes Wort ab vier Zeichen aus diesen Namen, als ganzes Wort und ohne Rücksicht
//!   auf Groß-/Kleinschreibung. So fällt auch ein halb gescrubbter Name auf („PRAXIS [PERSON]
//!   ERFUNDEN"). Ausgenommen sind `FIKTIV` (steht in jedem erfundenen Namen, auch in Firmennamen) und
//!   Rechtsformen.
//! - **Kontonummern** — die Kontonummer jeder IBAN (ab Zeichen 13, ohne führende Nullen, ab vier
//!   Ziffern), als ganze Ziffernfolge. Normalisiert verglichen träfe sie jeden Betrag mit denselben
//!   Ziffern.
//!
//! Firmennamen **mit** Gläubiger-ID sind öffentlicher Analyse-Anker und bleiben erlaubt. Vor dem
//! Vergleich werden HTML-Entities und JSON-`\u`-Escapes aufgelöst — ein escapter Name ist ein
//! geleakter Name.

use std::collections::BTreeSet;
use std::path::Path;

use crate::config::{PersonenConfig, SeedConfig};
use crate::run::RunPaths;

/// Die Spalten des CSV-CAMT-v8-Exports, aus denen verbotene Werte stammen.
const AUFTRAGSKONTO: usize = 0;
const GLAEUBIGER_ID: usize = 5;
const MANDATSREFERENZ: usize = 6;
const KUNDENREFERENZ: usize = 7;
const EMPFAENGER: usize = 11;
const IBAN: usize = 12;
const BIC: usize = 13;

/// Ab hier steht in einer deutschen IBAN die Kontonummer (Zeichen 13, nullbasiert 12).
const KONTONUMMER_AB: usize = 12;
/// Kürzere Werte prüft die Liste nicht — Platzhalter (`0`) und kurze Ziffernfolgen träfen sonst
/// beliebigen Text.
const MINDESTLAENGE: usize = 4;
/// Platzhalter der Bank, kein Rohwert.
const PLATZHALTER: [&str; 1] = ["NOTPROVIDED"];
/// Namenswörter, die keine Person verraten.
const KEIN_NAMENSWORT: [&str; 6] = ["FIKTIV", "GMBH", "AG", "EG", "SE", "MBH"];

/// Die verbotenen Werte eines oder mehrerer Läufe (siehe Modul-Doku).
#[derive(Debug, Default)]
pub(crate) struct VerboteneWerte {
    ganze: BTreeSet<String>,
    namenswoerter: BTreeSet<String>,
    kontonummern: BTreeSet<String>,
}

impl VerboteneWerte {
    /// Liest die verbotenen Werte aus den Eingaben eines Laufs: dem Auszug, der Eigene-Konten-Config,
    /// der Namensliste und der Seed-Config.
    pub(crate) fn aus_lauf(paths: &RunPaths) -> Self {
        let mut werte = Self::default();
        werte.aus_auszug(&paths.transactions);
        werte.aus_eigenen_konten(&paths.own_accounts_config);
        PersonenConfig::load(&paths.persons_config)
            .expect("die Namensliste lädt")
            .into_names()
            .iter()
            .for_each(|name| werte.name(name));
        werte.ganz(
            &SeedConfig::load(&paths.seed_config)
                .expect("die Seed-Config lädt")
                .seed()
                .to_string(),
        );
        werte
    }

    /// Vereinigt die verbotenen Werte zweier Läufe.
    pub(crate) fn und(mut self, andere: Self) -> Self {
        self.ganze.extend(andere.ganze);
        self.namenswoerter.extend(andere.namenswoerter);
        self.kontonummern.extend(andere.kontonummern);
        self
    }

    /// Steht `wert` auf der Liste — als ganzer Wert, Namenswort oder Kontonummer? Für die
    /// Gegenprobe des Tests, dass die Liste überhaupt etwas prüft.
    pub(crate) fn enthaelt(&self, wert: &str) -> bool {
        self.ganze.contains(wert)
            || self.namenswoerter.contains(wert)
            || self.kontonummern.contains(wert)
    }

    /// Wie viele Werte die Liste prüft.
    pub(crate) fn anzahl(&self) -> usize {
        self.ganze.len() + self.namenswoerter.len() + self.kontonummern.len()
    }

    /// Prüft eine Ausgabe (`ausgabe` benennt sie in der Meldung): kein verbotener Wert, kein
    /// Konto-Token.
    pub(crate) fn pruefe(&self, ausgabe: &str, text: &str) {
        let text = dekodiert(text);
        // Ein Konto-Token ist kein Rohwert, aber auch er bleibt drinnen — nur roh geprüft:
        // normalisiert träfe „KONTO" schon jede Spaltenüberschrift.
        assert!(!text.contains("KONTO_"), "{ausgabe} trägt ein Konto-Token");
        let normal = normalisiert(&text);
        for wert in &self.ganze {
            assert!(!text.contains(wert.as_str()), "{ausgabe} trägt {wert}");
            assert!(
                !normal.contains(&normalisiert(wert)),
                "{ausgabe} trägt {wert} (normalisiert)"
            );
        }
        let woerter = woerter(&text).collect::<BTreeSet<_>>();
        for wort in &self.namenswoerter {
            assert!(
                !woerter.contains(wort),
                "{ausgabe} trägt das Namenswort {wort}"
            );
        }
        let ziffernfolgen = text
            .split(|zeichen: char| !zeichen.is_ascii_digit())
            .map(|folge| folge.trim_start_matches('0'))
            .collect::<BTreeSet<_>>();
        for nummer in &self.kontonummern {
            assert!(
                !ziffernfolgen.contains(nummer.as_str()),
                "{ausgabe} trägt die Kontonummer {nummer}"
            );
        }
    }

    fn aus_auszug(&mut self, csv: &Path) {
        let text = std::fs::read_to_string(csv).expect("der Auszug ist lesbar");
        for zeile in text.lines().skip(1) {
            let zellen = zeile
                .split(';')
                .map(|zelle| zelle.trim_matches('"'))
                .collect::<Vec<_>>();
            let zelle = |index: usize| zellen.get(index).copied().unwrap_or_default();
            for iban in [zelle(AUFTRAGSKONTO), zelle(IBAN)] {
                self.iban(iban);
            }
            for wert in [zelle(BIC), zelle(MANDATSREFERENZ), zelle(KUNDENREFERENZ)] {
                self.ganz(wert);
            }
            if zelle(GLAEUBIGER_ID).is_empty() {
                self.name(zelle(EMPFAENGER));
            }
        }
    }

    /// Die IBANs der Eigene-Konten-Config — so geschrieben, wie sie in der Datei stehen.
    fn aus_eigenen_konten(&mut self, toml_datei: &Path) {
        let tabelle: toml::Table = toml::from_str(
            &std::fs::read_to_string(toml_datei).expect("die Konten-Config ist lesbar"),
        )
        .expect("die Konten-Config ist gültiges TOML");
        tabelle
            .get("konto")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|konto| konto.get("iban").and_then(toml::Value::as_str))
            .for_each(|iban| self.iban(iban));
    }

    fn ganz(&mut self, wert: &str) {
        let wert = wert.trim();
        if wert.chars().count() >= MINDESTLAENGE && !PLATZHALTER.contains(&wert) {
            self.ganze.insert(wert.to_string());
        }
    }

    /// Eine IBAN: als ganzer Wert (wie geschrieben und kompakt) und ihre Kontonummer.
    fn iban(&mut self, iban: &str) {
        self.ganz(iban);
        let kompakt = iban
            .chars()
            .filter(|zeichen| !zeichen.is_whitespace())
            .collect::<String>();
        self.ganz(&kompakt);
        let nummer = kompakt
            .chars()
            .skip(KONTONUMMER_AB)
            .collect::<String>()
            .trim_start_matches('0')
            .to_string();
        if nummer.len() >= MINDESTLAENGE && nummer.chars().all(|zeichen| zeichen.is_ascii_digit()) {
            self.kontonummern.insert(nummer);
        }
    }

    /// Ein Personen-Name: ganz und Wort für Wort.
    fn name(&mut self, name: &str) {
        self.ganz(name);
        self.namenswoerter.extend(
            woerter(name)
                .filter(|wort| wort.chars().count() >= MINDESTLAENGE)
                .filter(|wort| !KEIN_NAMENSWORT.contains(&wort.as_str())),
        );
    }
}

/// Die Wörter eines Texts, großgeschrieben — zerlegt an allem, was kein Buchstabe und keine Ziffer
/// ist. Ein Treffer ist darum immer ein **ganzes** Wort.
fn woerter(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|zeichen: char| !zeichen.is_alphanumeric())
        .filter(|wort| !wort.is_empty())
        .map(str::to_uppercase)
}

/// Großgeschrieben und nur noch Buchstaben und Ziffern.
fn normalisiert(text: &str) -> String {
    text.chars()
        .filter(|zeichen| zeichen.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Löst JSON-`\u`-Escapes und die HTML-Entities des Bericht-Escapers auf (`&amp;` zuletzt, damit
/// `&amp;lt;` nicht zu `<` wird).
fn dekodiert(text: &str) -> String {
    [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&amp;", "&"),
    ]
    .iter()
    .fold(ohne_json_escapes(text), |text, (entity, zeichen)| {
        text.replace(entity, zeichen)
    })
}

/// Ersetzt jedes `\uXXXX` durch sein Zeichen; ein unvollständiges oder ungültiges bleibt stehen.
fn ohne_json_escapes(text: &str) -> String {
    let mut ergebnis = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((vorher, nachher)) = rest.split_once("\\u") {
        ergebnis.push_str(vorher);
        let escape = nachher
            .get(..4)
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(char::from_u32)
            .zip(nachher.get(4..));
        if let Some((zeichen, danach)) = escape {
            ergebnis.push(zeichen);
            rest = danach;
        } else {
            ergebnis.push_str("\\u");
            rest = nachher;
        }
    }
    ergebnis.push_str(rest);
    ergebnis
}
