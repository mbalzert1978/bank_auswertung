//! Der **toml-Loader der Eigene-Konten-Config** ([Issue 0002](../../../../docs/issues/0002-eigene-konten-config-schema-und-loader.md),
//! [ADR-0013](../../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)
//! Entscheidung F).
//!
//! Er schließt die **eine** Lücke zwischen der Datei, die der Nutzer pflegt, und dem Wert, den der
//! `auswerten`-Slice entgegennimmt: die Datei führt **rohe IBANs** (das ist der einzige Ort, an dem
//! sie stehen dürfen — lokal, gitignored), der Slice bekommt ausschließlich **Token**
//! ([`OwnAccountsConfig`]). Zwischen beidem liegt genau dieser Loader:
//!
//! 1. **lesen + deuten** (`[[konto]]`-Blöcke mit `iban` und `rolle`),
//! 2. **ablehnen, was nicht trägt** — eine strukturell oder per **mod 97** ungültige IBAN, eine
//!    unbekannte Rolle, ein Duplikat. Nichts wird still übersprungen: ein stillschweigend
//!    verworfenes eigenes Konto liefe später als *fremd* durch, und die Umbuchung darauf sähe wie
//!    Konsum aus,
//! 3. **tokenisieren** mit **demselben** Obfuskator und **demselben** Seed, mit denen `obfuskieren`
//!    die Gegenkonto-Token gebildet hat ([`account_token`]), und den Seed-Marker derselben Ableitung
//!    mitgeben, die der `auswerten`-Lese-Seam aus dem abgelegten Seed zieht. Damit passen Fenster-
//!    und Config-Marker **per Konstruktion** zusammen, statt sich später zufällig zu treffen.
//!
//! **Die rohe IBAN endet hier.** [`EigeneKontenConfig`] hält sie im Prozess, gibt sie aber nirgends
//! heraus: sie hat keinen Getter, ihre [`Debug`](std::fmt::Debug)-Impl ist von Hand **redigiert**
//! (wie bei [`SeedConfig`](super::SeedConfig)), und der einzige Ausgang
//! [`to_own_accounts_config`](EigeneKontenConfig::to_own_accounts_config) liefert nur Token. Was `io`
//! weiterreicht, kann darum keine IBAN mehr sein.
//!
//! **Fehlbar + redigiert:** jeder Fehlschlag läuft über den geteilten
//! [`ConfigError`](super::ConfigError) — **kein Panic**, und **nie** ein Rohwert im Fehlertext (weder
//! Pfad noch Dateiinhalt noch die abgelehnte IBAN). Gerade hier wiegt das schwer: der Rohwert, der
//! zur Ablehnung führte, **ist** die eigene IBAN.

use std::collections::BTreeSet;
use std::path::Path;

use auswerten_adapter::{OwnAccountRow, OwnAccountsConfig, account_role_label};
use obfuskierung_adapter::account_token;

use super::{ConfigError, ConfigKind};
use crate::auswerten::seed_marker;
use crate::obfuskierung::SeededObfuscator;

/// Ein eigenes Konto, wie es in der Datei steht: die **rohe** IBAN (normalisiert) und die
/// **kanonische** Rolle. Crate-privat und ohne Getter auf die IBAN — sie verlässt diesen Loader nur
/// als Token.
struct OwnAccount {
    /// Die IBAN, kompakt und großgeschrieben — die Form, in der auch eine CAMT-Zeile sie führt, und
    /// damit die Form, über die der Obfuskator dasselbe Token bildet.
    iban: String,
    /// Die kanonische Rollen-Beschriftung, wie `auswerten`s Vokabular sie vergibt.
    role: &'static str,
}

/// Die geladene Eigene-Konten-Config als reiner Wert-Typ: die eigenen Konten mit ihrer Rolle.
///
/// **Kein `Debug`-Derive:** die von Hand geschriebene Impl redigiert die IBANs. Ein `{:?}` in einer
/// Panik- oder Log-Zeile ist sonst genau der Weg, auf dem ein struktureller Identifikator die
/// Maschine doch noch verließe.
pub struct EigeneKontenConfig {
    accounts: Vec<OwnAccount>,
}

impl std::fmt::Debug for EigeneKontenConfig {
    /// Redigiert: es erscheint nur die **Anzahl** der Konten, nie eine IBAN und nie eine Rolle.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EigeneKontenConfig")
            .field("konten", &self.accounts.len())
            .finish()
    }
}

impl EigeneKontenConfig {
    /// Lädt die Config aus einer Datei. Ein unlesbarer/fehlender Pfad ist ein **redigierter**
    /// [`ConfigError::Unreadable`](super::ConfigError::Unreadable) — der Pfad selbst bleibt draußen.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        // Die io-Fehlermeldung wird bewusst **verworfen** (nicht als `source` gehängt): sie könnte den
        // Pfad tragen, und redigiert heißt kein Rohwert im Fehler-Chain.
        let raw = std::fs::read_to_string(path)
            .map_err(|_err| ConfigError::Unreadable(ConfigKind::EigeneKonten))?;
        Self::from_toml_str(&raw)
    }

    /// Parst die Config aus einem toml-Text. Öffentlich, damit der Test sie gegen eine eingebettete
    /// Fixture prüfen kann, ohne über das Dateisystem zu gehen.
    ///
    /// Eine Config **ohne** ein einziges Konto wird abgelehnt: sie wäre keine Konfiguration, sondern
    /// die stille Zusage, dass es kein eigenes Konto gibt — und jede Umbuchung liefe als fremd durch.
    pub fn from_toml_str(raw: &str) -> Result<Self, ConfigError> {
        // Der toml-Parsefehler wird verworfen — er kann Zeilen-/Feldinhalt (bis hin zur IBAN)
        // zitieren; redigiert heißt, nur die statische Ursache reist weiter.
        let table: toml::Table = raw
            .parse()
            .map_err(|_err| ConfigError::Malformed(ConfigKind::EigeneKonten))?;

        let entries = table
            .get("konto")
            .and_then(toml::Value::as_array)
            .ok_or(ConfigError::OwnAccountsMissing)?;

        let accounts = entries
            .iter()
            .map(OwnAccount::from_entry)
            .collect::<Result<Vec<_>, _>>()?;

        match accounts.is_empty() {
            true => Err(ConfigError::OwnAccountsMissing),
            false => Self::without_duplicates(accounts),
        }
    }

    /// Lehnt eine zweimal geführte IBAN ab. Sie wäre nicht bloß redundant: zwei Zeilen könnten
    /// **verschiedene** Rollen tragen, und welche gälte, entschiede dann die Reihenfolge in der Datei.
    fn without_duplicates(accounts: Vec<OwnAccount>) -> Result<Self, ConfigError> {
        let unique = accounts
            .iter()
            .map(|account| account.iban.as_str())
            .collect::<BTreeSet<_>>();
        match unique.len() == accounts.len() {
            true => Ok(Self { accounts }),
            false => Err(ConfigError::OwnAccountDuplicate),
        }
    }

    /// Tokenisiert die Konten zu der Config, die der `auswerten`-Slice als **Wert** entgegennimmt —
    /// der einzige Ausgang dieses Typs, und er führt **keine** IBAN mehr.
    ///
    /// Der Seed-Marker ist dieselbe Ableitung, die der `auswerten`-Lese-Seam aus dem je Zeile
    /// abgelegten Seed zieht: stammen Fenster und Config aus **einem** Seed, stimmen die Marker
    /// überein — sonst scheitert der Lauf terminal, statt jede Umbuchung lautlos als fremd zu führen
    /// (ADR-0010).
    #[must_use]
    pub fn to_own_accounts_config(&self, seed: u64) -> OwnAccountsConfig {
        OwnAccountsConfig {
            seed_marker: seed_marker(seed),
            accounts: self
                .accounts
                .iter()
                .map(|account| OwnAccountRow {
                    // `account_token` gibt `None` nur für einen leeren Klartext zurück — den hat der
                    // Rand bereits ausgeschlossen (`OwnAccount::from_entry` lässt keine leere IBAN
                    // durch). Der Default ist damit unerreichbar und hält den Pfad trotzdem panikfrei;
                    // ein leeres Token lehnte der Slice ohnehin als fehlendes Pflichtfeld ab.
                    token: account_token(seed, &account.iban, &SeededObfuscator)
                        .unwrap_or_default(),
                    role: account.role.to_string(),
                })
                .collect(),
        }
    }
}

impl OwnAccount {
    /// Deutet einen `[[konto]]`-Block „außen → innen". Beide Pflichtfelder sind fehlbar, keins wird
    /// still übersprungen.
    fn from_entry(entry: &toml::Value) -> Result<Self, ConfigError> {
        let iban = entry
            .get("iban")
            .and_then(toml::Value::as_str)
            .ok_or(ConfigError::OwnAccountIbanMissing)?;
        let role = entry
            .get("rolle")
            .and_then(toml::Value::as_str)
            .ok_or(ConfigError::OwnAccountRoleMissing)?;

        Ok(Self {
            iban: normalised_iban(iban).ok_or(ConfigError::OwnAccountIbanInvalid)?,
            role: account_role_label(role).ok_or(ConfigError::OwnAccountRoleUnknown)?,
        })
    }
}

/// Normalisiert eine IBAN-Zelle (Leerzeichen raus, Großschreibung) und gibt sie nur zurück, wenn sie
/// **strukturell** und per **mod 97** gültig ist — `None` sonst. _Parse, don't validate_: ab hier
/// trägt der Wert seinen Beweis, und der Slice prüft nichts nach (er sieht nur noch das Token).
///
/// Die kompakte Großform ist bewusst die gespeicherte: eine CAMT-Zeile führt die IBAN genauso, und
/// nur dann bildet derselbe Obfuskator aus Config-IBAN und Buchungs-IBAN **dasselbe** Token.
fn normalised_iban(raw: &str) -> Option<String> {
    let compact: String = raw
        .chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| character.to_ascii_uppercase())
        .collect();
    (well_formed(&compact) && checksum_holds(&compact)).then_some(compact)
}

/// Die **Form** einer IBAN: 15–34 Zeichen, zwei Buchstaben Land, zwei Ziffern Prüfziffer, danach nur
/// Alphanumerisches. Bewusst länder-generisch — die Längentabelle je Land wäre externe
/// Referenzinformation und gehörte damit hinter einen eigenen Port, nicht in diesen Loader; die
/// Prüfsumme fängt praktisch alles ab, was diese Form passiert.
fn well_formed(iban: &str) -> bool {
    let mut characters = iban.chars();
    let country = [characters.next(), characters.next()];
    let check = [characters.next(), characters.next()];
    (15..=34).contains(&iban.chars().count())
        && country
            .iter()
            .all(|character| character.is_some_and(|c| c.is_ascii_uppercase()))
        && check
            .iter()
            .all(|character| character.is_some_and(|c| c.is_ascii_digit()))
        && characters.all(|character| character.is_ascii_alphanumeric())
}

/// Die **mod-97-Prüfsumme** (ISO 13616): die ersten vier Zeichen nach hinten rotieren, Buchstaben zu
/// zwei Ziffern (`A` = 10 … `Z` = 35) auflösen und stückweise `mod 97` rechnen — der Rest muss `1`
/// sein. Stückweise, weil die Zahl sonst bis zu 34-stellig würde und in keinen Integer passte.
fn checksum_holds(iban: &str) -> bool {
    let (head, tail) = iban.split_at_checked(4).unwrap_or((iban, ""));
    tail.chars()
        .chain(head.chars())
        .try_fold(0_u32, |remainder, character| {
            let digits = character.to_digit(36)?;
            let base = if digits < 10 { 10 } else { 100 };
            Some((remainder * base + digits) % 97)
        })
        == Some(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Eine erfundene, prüfsummen-gültige Config — dieselben Konten, die die Lauf-Fixture führt. Die
    /// erste IBAN steht bewusst **gruppiert**, die zweite kompakt: beide Schreibweisen müssen laden.
    const GUELTIG: &str = r#"
[[konto]]
iban  = "DE93 1000 0000 0000 0001 00"
rolle = "Giro"

[[konto]]
iban  = "DE12100000000000000200"
rolle = "haushalt"
"#;

    /// Die erste IBAN der Fixture in kompakter Form — als **erwarteter Rohwert**, der in keiner
    /// Ausgabe auftauchen darf.
    const ROH_IBAN: &str = "DE93100000000000000100";

    #[test]
    fn eine_gueltige_config_liefert_token_und_rollen_aber_nie_eine_iban() {
        let config = EigeneKontenConfig::from_toml_str(GUELTIG).expect("gültige Config lädt");

        let tokenised = config.to_own_accounts_config(42);

        assert_eq!(
            tokenised
                .accounts
                .iter()
                .map(|account| account.role.as_str())
                .collect::<Vec<_>>(),
            vec!["giro", "haushalt"],
            "die Rolle wird kanonisch abgelegt, unabhängig von der Schreibweise in der Datei"
        );
        assert!(
            tokenised
                .accounts
                .iter()
                .all(|account| account.token.starts_with("KONTO_")),
            "die Config trägt Token, keine IBANs"
        );
        assert!(
            !format!("{tokenised:?}").contains(ROH_IBAN),
            "keine Roh-IBAN in der tokenisierten Config"
        );
    }

    #[test]
    fn das_token_ist_genau_das_der_pipeline() {
        // Die eine Zusage, auf der die `Eigen`-Auflösung ruht: dasselbe Konto, derselbe Seed, dasselbe
        // Token wie das, das `obfuskieren` einer Buchung mit dieser Gegen-IBAN vergäbe.
        let config = EigeneKontenConfig::from_toml_str(GUELTIG).expect("gültige Config lädt");

        let erwartet = account_token(42, ROH_IBAN, &SeededObfuscator)
            .expect("nicht-leere IBAN ergibt ein Token");

        assert_eq!(
            config
                .to_own_accounts_config(42)
                .accounts
                .first()
                .map(|account| account.token.clone()),
            Some(erwartet)
        );
    }

    #[test]
    fn ein_anderer_seed_ergibt_einen_anderen_marker_und_andere_token() {
        let config = EigeneKontenConfig::from_toml_str(GUELTIG).expect("gültige Config lädt");

        let a = config.to_own_accounts_config(42);
        let b = config.to_own_accounts_config(43);

        assert_ne!(a.seed_marker, b.seed_marker);
        assert_ne!(a.accounts, b.accounts);
    }

    #[test]
    fn eine_iban_mit_falscher_pruefsumme_wird_abgelehnt_ohne_sie_zu_zitieren() {
        // Genau der Fall der Vorlage `eigene-konten.example.toml`: ihre Platzhalter sind absichtlich
        // prüfsummen-ungültig, damit eine unbefüllte Config laut scheitert statt still zu laden.
        let leck = "DE00000000000000000000";

        let fehler = EigeneKontenConfig::from_toml_str(&format!(
            "[[konto]]\niban = \"{leck}\"\nrolle = \"Giro\"\n"
        ))
        .expect_err("Prüfsummen-Fehler");

        assert_eq!(fehler, ConfigError::OwnAccountIbanInvalid);
        assert!(
            !fehler.to_string().contains(leck),
            "die Meldung darf die abgelehnte IBAN nicht zitieren"
        );
    }

    #[test]
    fn eine_unbekannte_rolle_wird_abgelehnt_statt_still_uebersprungen() {
        let fehler = EigeneKontenConfig::from_toml_str(&format!(
            "[[konto]]\niban = \"{ROH_IBAN}\"\nrolle = \"Tagesgeld\"\n"
        ))
        .expect_err("unbekannte Rolle");

        assert_eq!(fehler, ConfigError::OwnAccountRoleUnknown);
    }

    #[test]
    fn ein_fehlendes_pflichtfeld_wird_abgelehnt() {
        assert_eq!(
            EigeneKontenConfig::from_toml_str("[[konto]]\nrolle = \"Giro\"\n").map(|_| ()),
            Err(ConfigError::OwnAccountIbanMissing)
        );
        assert_eq!(
            EigeneKontenConfig::from_toml_str(&format!("[[konto]]\niban = \"{ROH_IBAN}\"\n"))
                .map(|_| ()),
            Err(ConfigError::OwnAccountRoleMissing)
        );
    }

    #[test]
    fn dieselbe_iban_zweimal_wird_abgelehnt() {
        // Zwei Zeilen könnten verschiedene Rollen tragen; welche gälte, entschiede sonst die
        // Reihenfolge in der Datei. Die zweite Schreibweise zeigt zugleich, dass die Normalisierung
        // vor dem Vergleich läuft.
        let fehler = EigeneKontenConfig::from_toml_str(&format!(
            "[[konto]]\niban = \"{ROH_IBAN}\"\nrolle = \"Giro\"\n\
             [[konto]]\niban = \"DE93 1000 0000 0000 0001 00\"\nrolle = \"Spar\"\n"
        ))
        .expect_err("Duplikat");

        assert_eq!(fehler, ConfigError::OwnAccountDuplicate);
    }

    #[test]
    fn eine_config_ohne_konten_wird_abgelehnt() {
        assert_eq!(
            EigeneKontenConfig::from_toml_str("# leer\n").map(|_| ()),
            Err(ConfigError::OwnAccountsMissing)
        );
        assert_eq!(
            EigeneKontenConfig::from_toml_str("konto = []\n").map(|_| ()),
            Err(ConfigError::OwnAccountsMissing)
        );
    }

    #[test]
    fn kaputtes_toml_und_eine_fehlende_datei_scheitern_redigiert() {
        assert_eq!(
            EigeneKontenConfig::from_toml_str("[[konto]\niban =").map(|_| ()),
            Err(ConfigError::Malformed(ConfigKind::EigeneKonten))
        );
        assert_eq!(
            EigeneKontenConfig::load(Path::new("existiert-nicht.toml")).map(|_| ()),
            Err(ConfigError::Unreadable(ConfigKind::EigeneKonten))
        );
    }

    #[test]
    fn die_debug_ausgabe_redigiert_die_ibans() {
        let config = EigeneKontenConfig::from_toml_str(GUELTIG).expect("gültige Config lädt");

        let ausgabe = format!("{config:?}");

        assert!(!ausgabe.contains("DE93"), "keine IBAN in der Debug-Ausgabe");
        assert!(ausgabe.contains('2'), "die Anzahl darf erscheinen");
    }
}
