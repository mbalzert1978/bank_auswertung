//! **Das Programm** ([Issue 0083](../../../docs/issues/0083-lauf-anstoss-binary-reiht-den-ausloeser-ein-und-dreht-die-broker-loop.md)):
//! die dünnste denkbare Hülle um [`io::run::run`]. Sie liest acht Pfade von der Kommandozeile, fährt
//! **einen** Lauf und setzt den Exit-Code — mehr tut sie nicht. Den HTML-Bericht (Issue 0012) schreibt
//! der Lauf selbst, und nur, wenn er gelingt. Dazu kommt seit Issue 0011 der Unterbefehl
//! `kategorien-import`, der eine Chat-Antwort über [`io::run::import_categories`] ins
//! Kategorie-Register übernimmt, und die optionale Option `--kategorien-export`.
//!
//! **Warum so wenig.** Die Verdrahtung der Pipeline gehört in die Infrastruktur, nicht in die
//! Kommandozeile: sie ist derselbe Lauf, ob ihn ein Terminal, ein Test oder später ein Scheduler
//! anstößt. Diese Crate bindet darum als einzige interne Abhängigkeit `io` — und keine externe, auch
//! keinen Argument-Parser: acht benannte Pflicht-Optionen zu lesen ist keine Bibliothek wert.
//!
//! **Das Lauf-Protokoll geht auf den Fehlerkanal**, die Standardausgabe bleibt frei. Es ist redigiert (siehe [`io::run`]): Nachrichtentypen, Zählwerte und im
//! Fehlerfall die bereits redigierte `Display` des Slice-Fehlers — nie ein Rohwert, nie Freitext, nie
//! eine IBAN, nie der Seed.

use std::path::PathBuf;
use std::process::ExitCode;

use io::auswerten::chat_import::ImportMode;
use io::run::{ImportPaths, RunPaths, import_categories, run};

/// Exit-Code eines gescheiterten Laufs — auch eines, der eine Nachricht aussondern musste.
const FEHLGESCHLAGEN: u8 = 1;
/// Exit-Code einer unbrauchbaren Kommandozeile — bewusst ein **anderer** als der des gescheiterten
/// Laufs: „falsch aufgerufen" ist kein Befund über die Daten.
const FALSCH_AUFGERUFEN: u8 = 2;

fn main() -> ExitCode {
    let mut protocol = std::io::stderr();

    let befehl = match parse(std::env::args().skip(1)) {
        Ok(befehl) => befehl,
        Err(fehler) => {
            eprintln!("{fehler}\n\n{USAGE}");
            return ExitCode::from(FALSCH_AUFGERUFEN);
        }
    };

    let ergebnis = match &befehl {
        Befehl::Lauf(paths) => run(paths, &mut protocol).map(drop),
        Befehl::KategorienImport { paths, mode } => {
            import_categories(paths, *mode, &mut protocol).map(drop)
        }
    };
    match ergebnis {
        Ok(()) => ExitCode::SUCCESS,
        Err(fehler) => {
            eprintln!("{} gescheitert: {fehler}", befehl.name());
            ExitCode::from(FEHLGESCHLAGEN)
        }
    }
}

/// Was die Kommandozeile verlangt: einen **Lauf** oder den **Import** einer Chat-Antwort.
#[derive(Debug)]
enum Befehl {
    Lauf(RunPaths),
    KategorienImport {
        paths: ImportPaths,
        mode: ImportMode,
    },
}

impl Befehl {
    fn name(&self) -> &'static str {
        match self {
            Self::Lauf(_) => "Lauf",
            Self::KategorienImport { .. } => "Kategorien-Import",
        }
    }
}

/// Der Unterbefehl für den Import einer Chat-Antwort.
const KATEGORIEN_IMPORT: &str = "kategorien-import";

const USAGE: &str = "\
Aufruf: bank-auswertung --db <datei> --csv <datei> --fenster <datei> --seed <datei> \
--eigene-konten <datei> --personen <datei> --sparen <datei> --bericht <datei.html>

  --db              SQLite-Datei (Outbox + Ergebnis-Tabellen); wird bei Bedarf angelegt
  --csv             CSV-CAMT-v8-Kontoauszug, den `einlesen` beschafft
  --fenster         Fenster-Config (window_days)
  --seed            Seed-Config (seed)
  --eigene-konten   Eigene-Konten-Config ([[konto]] mit iban + rolle)
  --personen        Namensliste (namen = [...]) für den Verwendungszweck: Namen, die das
                    Programm nicht selbst kennt — auch Vornamen/Kurzformen von Empfängern;
                    `namen = []`, wenn es keine gibt. Vor dem ersten Lauf füllen: spätere
                    Namen wirken nicht rückwirkend. Vorlage: personen.example.toml
  --sparen          Spar-Config (jahresbetrag_ab_cent): ab diesem Jahresbetrag in Cent wird ein
                    Kostentreiber als Spar-Kandidat vorgeschlagen — ein Vorschlag, entschieden
                    wird von dir. Vorlage: sparen.example.toml
  --bericht         Ziel des HTML-Berichts. Er entsteht nur, wenn der Lauf gelingt (Exit 0),
                    und zeigt genau diesen Lauf; eine ältere Datei dort wird ersetzt. Endet
                    der Lauf mit einem Fehler, bleibt sie unberührt.

Alle acht sind Pflicht. Das Lauf-Protokoll geht auf den Fehlerkanal.

  --kategorien-export <datei>
                    optional: schreibt die Buchungsarten, die das Kategorie-Register noch nicht
                    kennt, als JSON für einen Chat (nur bei einem gelungenen Lauf, Rechte 0600).
                    Hinaus gehen nur Gläubiger-IDs, Firmennamen und bei Zahlungsdiensten (PayPal,
                    Amazon, Klarna) der Verwendungszweck — ohne Wörter mit Ziffer oder @. Personen
                    und Kartenzahlungen fehlen bewusst. Der Chat-Anbieter sieht damit, welche
                    Firmen bei dir einziehen. Die Datei VOR dem Einfügen ansehen.

Aufruf: bank-auswertung kategorien-import --db <datei> --antwort <datei> [--korrektur]

  --db              dieselbe SQLite-Datei wie beim Lauf
  --antwort         die Antwort aus dem Chat: {\"antworten\": [{\"fingerabdruck\": …, \"kategorie\": …}]}
  --korrektur       ersetzt auch schon bekannte Kategorien; ohne den Schalter bleiben sie

Zugeordnet wird nur über den exakten Fingerabdruck; weitere Felder werden nie gelesen.
Exit 1, wenn die Datei abgewiesen wurde oder nichts gelernt, aber etwas verworfen wurde.

Scheitert eine Nachricht, wird sie ausgesondert: der Lauf nennt sie am Ende und endet mit
Exit 1, die übrigen laufen weiter. Ein späterer Lauf wird von ihr nicht aufgehalten; seine
Schlussbilanz zählt sie als „früher ausgesondert“. Fehlt der Analyse deshalb eine Buchung,
steht dort „Analyse unvollständig“ — auch bei Exit 0. Neu einlesen holt sie nicht zurück
(eine bekannte Zeile wird nie neu publiziert) — dann hilft nur ein Redrive per SQL oder eine
frische --db, siehe ADR-0017. Ein ausgesonderter Import-Auslöser dagegen: Auszug neu einlesen.

Der Verwendungszweck wird vor der Weitergabe gescrubbt: IBANs, BICs und bekannte Namen
(die vollständigen Namen aller Empfänger ohne gültige Gläubiger-ID plus --personen) werden
ersetzt. Das ist heuristisch und nicht beweisbar vollständig — ein Name, der nirgends bekannt
ist, bleibt stehen. In --personen gehören darum auch PayPal-Freunde, Vornamen, der eigene Name
und Personen mit eigener Gläubiger-ID (privater Vermieter, Verein).

Mehrere Läufe in dieselbe --db sind gewollt: je eigenem Konto ein Lauf, auch wiederholt.
Ein neuer Auszug ergänzt den Bestand; eine schon eingelesene Zeile wird an ihrem
Zeilen-Schlüssel wiedererkannt und nicht erneut gezählt. Jeder Lauf wertet alle Konten
der --db gemeinsam aus; maßgeblich ist der jüngste Lauf. Vorgemerkte Umsätze werden
übersprungen und kommen gebucht mit einem späteren Auszug.
GRENZE: ein Tag muss vollständig im Export stehen — überlappen sich zwei Auszüge, muss
jeder den gemeinsamen Tag ganz tragen. Sonst können gleiche Buchungen dieses Tages als
schon bekannt gelten und verloren gehen (ADR-0016).";

/// Warum die Kommandozeile nicht trägt. **Redigiert wie alles andere:** die Meldung nennt die
/// **Option**, nie den übergebenen Wert — ein Pfad kann Kontonamen tragen, und ein verrutschtes
/// Argument könnte alles Mögliche sein.
#[derive(Debug, PartialEq, Eq)]
enum Aufruffehler {
    UnbekannteOption(String),
    WertFehlt(&'static str),
    OptionFehlt(&'static str),
}

impl std::fmt::Display for Aufruffehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnbekannteOption(option) => write!(f, "unbekannte Option: {option}"),
            Self::WertFehlt(option) => write!(f, "{option} braucht einen Pfad"),
            Self::OptionFehlt(option) => write!(f, "{option} fehlt"),
        }
    }
}

/// Die acht Optionen, während sie eingesammelt werden — jede noch **offen**, bis sie gesetzt ist. Der
/// Übergang „alle acht da" ist genau der Punkt, an dem daraus [`RunPaths`] wird
/// ([`vollstaendig`](Aufrufoptionen::vollstaendig)); vorher gibt es keinen Wert, der so tut, als wäre
/// er vollständig.
#[derive(Debug, Default)]
struct Aufrufoptionen {
    database: Option<PathBuf>,
    transactions: Option<PathBuf>,
    window_config: Option<PathBuf>,
    seed_config: Option<PathBuf>,
    own_accounts_config: Option<PathBuf>,
    persons_config: Option<PathBuf>,
    savings_config: Option<PathBuf>,
    /// Optional — fehlt sie, entsteht kein Export.
    category_export: Option<PathBuf>,
    report: Option<PathBuf>,
}

impl Aufrufoptionen {
    /// Das Feld hinter einer Option, **zusammen mit ihrem `'static`-Namen** — die eine Stelle, an der
    /// ein Optionsname auf sein Ziel trifft. Dass der Name aus demselben `match` fällt, erspart eine
    /// zweite Liste, die von dieser abweichen könnte: wer das Feld hat, hat auch den Namen für die
    /// Fehlermeldung.
    fn feld(&mut self, option: &str) -> Option<(&'static str, &mut Option<PathBuf>)> {
        match option {
            "--db" => Some(("--db", &mut self.database)),
            "--csv" => Some(("--csv", &mut self.transactions)),
            "--fenster" => Some(("--fenster", &mut self.window_config)),
            "--seed" => Some(("--seed", &mut self.seed_config)),
            "--eigene-konten" => Some(("--eigene-konten", &mut self.own_accounts_config)),
            "--personen" => Some(("--personen", &mut self.persons_config)),
            "--sparen" => Some(("--sparen", &mut self.savings_config)),
            "--kategorien-export" => Some(("--kategorien-export", &mut self.category_export)),
            "--bericht" => Some(("--bericht", &mut self.report)),
            _ => None,
        }
    }

    fn vollstaendig(self) -> Result<RunPaths, Aufruffehler> {
        Ok(RunPaths {
            database: self.database.ok_or(Aufruffehler::OptionFehlt("--db"))?,
            transactions: self
                .transactions
                .ok_or(Aufruffehler::OptionFehlt("--csv"))?,
            window_config: self
                .window_config
                .ok_or(Aufruffehler::OptionFehlt("--fenster"))?,
            seed_config: self
                .seed_config
                .ok_or(Aufruffehler::OptionFehlt("--seed"))?,
            own_accounts_config: self
                .own_accounts_config
                .ok_or(Aufruffehler::OptionFehlt("--eigene-konten"))?,
            persons_config: self
                .persons_config
                .ok_or(Aufruffehler::OptionFehlt("--personen"))?,
            savings_config: self
                .savings_config
                .ok_or(Aufruffehler::OptionFehlt("--sparen"))?,
            category_export: self.category_export,
            report: self.report.ok_or(Aufruffehler::OptionFehlt("--bericht"))?,
        })
    }
}

/// Die Optionen des Unterbefehls `kategorien-import`, während sie eingesammelt werden.
#[derive(Debug, Default)]
struct ImportOptionen {
    database: Option<PathBuf>,
    answers: Option<PathBuf>,
    correction: bool,
}

impl ImportOptionen {
    fn feld(&mut self, option: &str) -> Option<(&'static str, &mut Option<PathBuf>)> {
        match option {
            "--db" => Some(("--db", &mut self.database)),
            "--antwort" => Some(("--antwort", &mut self.answers)),
            _ => None,
        }
    }

    fn vollstaendig(self) -> Result<Befehl, Aufruffehler> {
        Ok(Befehl::KategorienImport {
            paths: ImportPaths {
                database: self.database.ok_or(Aufruffehler::OptionFehlt("--db"))?,
                answers: self.answers.ok_or(Aufruffehler::OptionFehlt("--antwort"))?,
            },
            mode: match self.correction {
                true => ImportMode::Correct,
                false => ImportMode::LearnUnknown,
            },
        })
    }
}

/// Liest die Kommandozeile: beginnt sie mit `kategorien-import`, den Import, sonst einen Lauf. Eine
/// doppelt gesetzte Option gewinnt in ihrer letzten Nennung — die übliche Konvention, und die einzige,
/// die ohne Sonderfall auskommt.
fn parse(args: impl Iterator<Item = String>) -> Result<Befehl, Aufruffehler> {
    let mut args = args.peekable();
    match args.next_if(|first| first == KATEGORIEN_IMPORT) {
        Some(_) => parse_import(args),
        None => parse_run(args).map(Befehl::Lauf),
    }
}

/// Liest die Optionen des Imports — `--korrektur` ist ein Schalter ohne Wert.
fn parse_import(args: impl Iterator<Item = String>) -> Result<Befehl, Aufruffehler> {
    let mut args = args;
    let mut optionen = ImportOptionen::default();

    while let Some(option) = args.next() {
        if option == "--korrektur" {
            optionen.correction = true;
            continue;
        }
        let Some((name, feld)) = optionen.feld(&option) else {
            return Err(Aufruffehler::UnbekannteOption(option));
        };
        match args.next() {
            Some(wert) => *feld = Some(PathBuf::from(wert)),
            None => return Err(Aufruffehler::WertFehlt(name)),
        }
    }

    optionen.vollstaendig()
}

/// Liest die acht Pflicht-Optionen eines Laufs und die optionale `--kategorien-export`.
fn parse_run(args: impl Iterator<Item = String>) -> Result<RunPaths, Aufruffehler> {
    let mut args = args;
    let mut optionen = Aufrufoptionen::default();

    while let Some(option) = args.next() {
        let Some((name, feld)) = optionen.feld(&option) else {
            return Err(Aufruffehler::UnbekannteOption(option));
        };
        // `name` ist die `'static`-Form der erkannten Option — ihn zu nennen leakt nichts.
        match args.next() {
            Some(wert) => *feld = Some(PathBuf::from(wert)),
            None => return Err(Aufruffehler::WertFehlt(name)),
        }
    }

    optionen.vollstaendig()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vollstaendige_argumente() -> Vec<String> {
        [
            "--db",
            "lauf.sqlite",
            "--csv",
            "auszug.csv",
            "--fenster",
            "fenster.toml",
            "--seed",
            "seed.toml",
            "--eigene-konten",
            "eigene-konten.toml",
            "--personen",
            "personen.toml",
            "--sparen",
            "sparen.toml",
            "--bericht",
            "bericht.html",
        ]
        .iter()
        .map(|argument| (*argument).to_string())
        .collect()
    }

    #[test]
    fn acht_optionen_ergeben_die_lauf_pfade() {
        let paths = parse_run(vollstaendige_argumente().into_iter()).expect("vollständiger Aufruf");

        assert_eq!(paths.database, PathBuf::from("lauf.sqlite"));
        assert_eq!(paths.transactions, PathBuf::from("auszug.csv"));
        assert_eq!(paths.window_config, PathBuf::from("fenster.toml"));
        assert_eq!(paths.seed_config, PathBuf::from("seed.toml"));
        assert_eq!(
            paths.own_accounts_config,
            PathBuf::from("eigene-konten.toml")
        );
        assert_eq!(paths.persons_config, PathBuf::from("personen.toml"));
        assert_eq!(paths.savings_config, PathBuf::from("sparen.toml"));
        assert_eq!(paths.report, PathBuf::from("bericht.html"));
        // Ohne die Option entsteht kein Export.
        assert_eq!(paths.category_export, None);
    }

    #[expect(
        clippy::panic,
        reason = "let-else-Bindung des erwarteten Befehls: schlägt der Test laut fehl statt still zu überspringen"
    )]
    #[test]
    fn die_export_option_ist_optional_und_traegt_ihr_ziel() {
        let mut argumente = vollstaendige_argumente();
        argumente.extend([
            "--kategorien-export".to_string(),
            "anfrage.json".to_string(),
        ]);

        let Ok(Befehl::Lauf(paths)) = parse(argumente.into_iter()) else {
            panic!("erwartete einen Lauf");
        };

        assert_eq!(paths.category_export, Some(PathBuf::from("anfrage.json")));
    }

    fn import_argumente(weitere: &[&str]) -> Vec<String> {
        [
            "kategorien-import",
            "--db",
            "lauf.sqlite",
            "--antwort",
            "antwort.json",
        ]
        .iter()
        .chain(weitere)
        .map(|argument| (*argument).to_string())
        .collect()
    }

    #[expect(
        clippy::panic,
        reason = "let-else-Bindung des erwarteten Befehls: schlägt der Test laut fehl statt still zu überspringen"
    )]
    #[test]
    fn der_unterbefehl_liest_datenbank_und_antwort_ohne_korrektur() {
        let Ok(Befehl::KategorienImport { paths, mode }) = parse(import_argumente(&[]).into_iter())
        else {
            panic!("erwartete den Import");
        };

        assert_eq!(paths.database, PathBuf::from("lauf.sqlite"));
        assert_eq!(paths.answers, PathBuf::from("antwort.json"));
        assert_eq!(mode, ImportMode::LearnUnknown);
    }

    #[expect(
        clippy::panic,
        reason = "let-else-Bindung des erwarteten Befehls: schlägt der Test laut fehl statt still zu überspringen"
    )]
    #[test]
    fn der_schalter_korrektur_erlaubt_das_ersetzen() {
        let Ok(Befehl::KategorienImport { mode, .. }) =
            parse(import_argumente(&["--korrektur"]).into_iter())
        else {
            panic!("erwartete den Import");
        };

        assert_eq!(mode, ImportMode::Correct);
    }

    #[test]
    fn der_import_ohne_antwort_nennt_die_fehlende_option() {
        let argumente = ["kategorien-import", "--db", "lauf.sqlite"]
            .iter()
            .map(|argument| (*argument).to_string());

        assert_eq!(
            parse(argumente).map(|_| ()),
            Err(Aufruffehler::OptionFehlt("--antwort"))
        );
    }

    #[test]
    fn der_import_kennt_keine_lauf_optionen() {
        assert_eq!(
            parse(import_argumente(&["--csv", "auszug.csv"]).into_iter()).map(|_| ()),
            Err(Aufruffehler::UnbekannteOption("--csv".to_string()))
        );
    }

    #[test]
    fn jede_fehlende_option_wird_benannt() {
        // Je Option den Aufruf **ohne** sie: der Fehler nennt genau die fehlende.
        for (index, option) in [
            "--db",
            "--csv",
            "--fenster",
            "--seed",
            "--eigene-konten",
            "--personen",
            "--sparen",
            "--bericht",
        ]
        .into_iter()
        .enumerate()
        {
            let ohne: Vec<String> = vollstaendige_argumente()
                .into_iter()
                .enumerate()
                .filter(|(position, _)| *position != index * 2 && *position != index * 2 + 1)
                .map(|(_, argument)| argument)
                .collect();

            assert_eq!(
                parse(ohne.into_iter()).map(|_| ()),
                Err(Aufruffehler::OptionFehlt(option))
            );
        }
    }

    #[test]
    fn eine_unbekannte_option_wird_abgelehnt() {
        assert_eq!(
            parse(["--report".to_string()].into_iter()).map(|_| ()),
            Err(Aufruffehler::UnbekannteOption("--report".to_string()))
        );
    }

    #[test]
    fn eine_option_ohne_wert_wird_abgelehnt() {
        assert_eq!(
            parse(["--db".to_string()].into_iter()).map(|_| ()),
            Err(Aufruffehler::WertFehlt("--db"))
        );
    }

    #[test]
    fn ein_leerer_aufruf_nennt_die_erste_fehlende_option() {
        assert_eq!(
            parse(std::iter::empty()).map(|_| ()),
            Err(Aufruffehler::OptionFehlt("--db"))
        );
    }
}
