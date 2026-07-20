use auswerten_domain::{AccountRole, AccountToken, NonEmptyString, OwnAccounts, SeedMarker};

/// Die **token-orientierte Eigene-Konten-Config**, wie sie dem Slice als **Wert** hereingereicht
/// wird (ADR-0013, Entscheidung F). Reines Eingangs-DTO an der Adapter-Grenze; die Abbildung
/// „außen → innen" auf die Domänen-[`OwnAccounts`] lebt hier (ACL).
///
/// **Sie trägt Token, keine IBANs.** Die rohe IBAN jedes eigenen Kontos existiert nur **vor** der
/// Tokenisierung — beim Loader ([Issue 0002](../../../../docs/issues/0002-eigene-konten-config-schema-und-loader.md)),
/// der sie prüft und mit **demselben `Obfuscator`/Seed** tokenisiert, mit dem `obfuskieren` die
/// Gegenkonto-Token der Buchungen gebildet hat. Genau deshalb ist der Vergleich hier tragfähig — und
/// genau deshalb sieht dieser Slice zu keinem Zeitpunkt eine IBAN, auch nicht als Zwischenwert.
///
/// **Kein serde:** die Config kommt als Wert aus der io-Verdrahtung, nicht aus einer Nachricht.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnAccountsConfig {
    /// Das **opake Kennzeichen der Tokenisierung**, unter der die Token dieser Config entstanden
    /// sind — dasselbe Etikett, das das Fenster trägt, wenn beide aus demselben Seed stammen.
    /// **Nicht** der Seed.
    pub seed_marker: String,
    pub accounts: Vec<OwnAccountRow>,
}

/// Ein **eigenes Konto** als rohe Config-Zeile: sein `KONTO_…`-Token und seine Rolle als noch zu
/// deutende Zeichenkette.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnAccountRow {
    /// Das seed-basierte Konto-Token — **nie** die IBAN, aus der es entstand.
    pub token: String,
    /// Die Rolle als rohe Zelle: `giro` | `haushalt` | `spar` | `vorlauf`.
    pub role: String,
}

impl OwnAccountsConfig {
    /// Deutet die Config in die Domänen-[`OwnAccounts`] (ACL, „außen → innen") — **hier** liegt die
    /// Ablehnung einer fehlerhaften Config, denn die Domäne **fordert** gültige Werte, sie prüft
    /// sie nicht (Repo-Konvention; ADR-0002/ADR-0005).
    ///
    /// Zwei Ausgänge sind terminal, keiner überspringt still eine Zeile: ein **leeres Token** ist
    /// ein fehlendes Pflichtfeld, und eine **unbekannte Rolle** ist keine Rolle. Beides
    /// stillschweigend zu ignorieren hieße, ein eigenes Konto klammheimlich als *fremd* zu führen —
    /// die Umbuchung darauf sähe wie Konsum aus, und niemand erführe davon.
    ///
    /// Die **IBAN prüft dieser Slice nicht**: sie ist hier nicht mehr vorhanden, und ihre Gültigkeit
    /// hat der Loader vor der Tokenisierung bereits geführt (_parse, don't validate_ — der Beweis
    /// ist geführt, das Token trägt ihn).
    pub(crate) fn to_own_accounts(&self) -> Result<OwnAccounts, ConfigRejected> {
        let marker = SeedMarker::from(NonEmptyString::new(&self.seed_marker).ok_or(
            ConfigRejected::MissingMandatoryField {
                field: "seed_marker",
            },
        )?);
        self.accounts
            .iter()
            .map(OwnAccountRow::to_entry)
            .collect::<Result<Vec<_>, _>>()
            .map(|roles| OwnAccounts::of(marker, roles))
    }
}

impl OwnAccountRow {
    /// Deutet eine Config-Zeile in das Paar (Token, Rolle), das die Eigen-Menge verlangt.
    fn to_entry(&self) -> Result<(AccountToken, AccountRole), ConfigRejected> {
        let token = NonEmptyString::new(&self.token)
            .ok_or(ConfigRejected::MissingMandatoryField { field: "token" })?;
        Ok((AccountToken::from(token), parse_role(&self.role)?))
    }
}

/// Deutet die rohe Rollen-Zelle auf den geschlossenen Summentyp. Getrimmt und
/// groß-/kleinschreibungs-tolerant — das ist **Mechanik** am Rand (wie das Trimmen des
/// `NonEmptyString`), keine fachliche Entscheidung; welche vier Rollen es gibt, sagt die Domäne.
fn parse_role(raw: &str) -> Result<AccountRole, ConfigRejected> {
    match raw.trim().to_lowercase().as_str() {
        "giro" => Ok(AccountRole::Giro),
        "haushalt" => Ok(AccountRole::Haushalt),
        "spar" => Ok(AccountRole::Spar),
        "vorlauf" => Ok(AccountRole::Vorlauf),
        _ => Err(ConfigRejected::UnknownAccountRole),
    }
}

/// Warum die Eigene-Konten-Config **abgelehnt** wird — der terminale Ausgang der
/// „außen → innen"-Abbildung, den der [`WindowEvaluationHandler`](crate::WindowEvaluationHandler)
/// auf den nackten Domänen-Fehler mappt.
///
/// **Redaktions-Invariante:** keine Variante trägt einen Rohwert. Beim fehlenden Pflichtfeld ist es
/// **nur** der statische Feldname; bei der unbekannten Rolle ist es **gar nichts** — eine
/// verrutschte Config-Zelle kann beliebigen Inhalt führen (im schlimmsten Fall eine dorthin
/// verirrte IBAN), und den zurückzuspiegeln wäre genau das Leck, das dieser Slice ausschließt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfigRejected {
    MissingMandatoryField { field: &'static str },
    UnknownAccountRole,
}
