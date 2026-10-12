//! **Der Lauf** ([Issue 0083](../../../../docs/issues/0083-lauf-anstoss-binary-reiht-den-ausloeser-ein-und-dreht-die-broker-loop.md)):
//! die eine Stelle, an der aus der Bibliothek ein Programm wird. Bis hierher lief die Pipeline
//! ausschließlich in Tests — [`run`] verdrahtet **alle** realen `io`-Adapter, reiht die Auslöser in
//! die Outbox und dreht die Broker-Schleife, bis keine offene Nachricht mehr bleibt.
//!
//! **Der Auslöser wird eingereiht, nicht aufgerufen.** Kein Slice wird direkt gerufen; die Kopplung
//! läuft weiter allein über die Outbox
//! ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Was dieses Modul
//! beisteuert, ist ausschließlich **Verdrahtung + Anstoß** — es kennt keine Fachlichkeit.
//!
//! ## Zwei Phasen, und warum
//!
//! Der Lauf zerfällt in **Import** (`ReadTransactions` → `Ingested` → `Obfuscated`) und **Analyse**
//! (`StartAnalysis` → `AdvanceWindow` → `WindowClosed` → `HistoryComplete`). Die beiden hängen
//! **nicht** über eine Nachricht aneinander, und das ist Absicht: `fenstern` paginiert die
//! **Gesamt**-Historie **einmal je Lauf**, nicht je Buchung. Ein `Obfuscated` in ein `StartAnalysis`
//! zu übersetzen hieße, die Analyse so oft anzustoßen, wie es Buchungen gibt.
//!
//! Daraus folgen die beiden einzigen Eigenheiten dieses Moduls:
//!
//! - Der **`StartAnalysis`-Anstoß kommt vom Lauf**, nicht von einem Slice — und **erst**, wenn der
//!   Import leer gelaufen ist. Früher gestellt sähe der Anstoß einen leeren `obfuskiert`-Bestand und
//!   beendete den Lauf lautlos ohne ein einziges Fenster.
//! - **`Obfuscated` hat keinen Abnehmer stromabwärts** (`fenstern` liest die *Tabelle*, nicht die
//!   Nachricht). Ohne einen Consumer bliebe jede dieser Nachrichten für immer offen und jeder
//!   spätere Lauf stellte sie erneut zu. Der [`AckSink`] räumt sie darum ab — er ist kein Slice und
//!   tut nichts als quittieren.
//!
//! ## Das Lauf-Protokoll ist redigiert
//!
//! Geschrieben wird je verarbeiteter Nachricht ihr **`message_type`** — ein statischer Dispatch-Key
//! —, am Ende die Zahl je Typ, und bei einem terminalen Fehlschlag dessen **`Display`** (der nackte,
//! bereits redigierte Domänen-Fehler des Slice). **Nie** ein Rohwert, **nie** Freitext, **nie** eine
//! IBAN, **nie** der Seed. Der `partition_key` bleibt bewusst draußen: er ist zwar nur ein opaker
//! io-Zeiger, aber die Ausgabe braucht ihn nicht, und was nicht gedruckt wird, kann nicht lecken.

use std::collections::BTreeMap;
use std::io::Write;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use auswerten_adapter::OwnAccountsConfig;
use obfuskierung_adapter::ObfuscatedMessage;

use crate::Dispatcher;
use crate::auswerten::{
    AuswertenSqliteStore, AuswertungsfensterSource, CONSUMED_HISTORY_MESSAGE_TYPE,
    CONSUMED_MESSAGE_TYPE as AUSWERTEN_CONSUMED, EmptyCategoryRegistry, UnansweredCategorizer,
    consumer as auswerten_consumer, history_consumer,
};
use crate::config::{
    AppConfig, ConfigError, EigeneKontenConfig, FensterConfig, PersonenConfig, SeedConfig,
};
use crate::einlesen::{
    CONSUMED_MESSAGE_TYPE as EINLESEN_CONSUMED, CsvCamtV8Source, EinlesenSqliteStore,
    GermanCalendar, GermanMoney, IsoCurrencies, consumer as einlesen_consumer,
};
use crate::fenstern::{
    CONSUMED_MESSAGE_TYPE as FENSTERN_CONSUMED, FensternSqliteStore, ObfuskiertWindowSource,
    START_MESSAGE_TYPE, consumer as fenstern_consumer, start_consumer,
};
use crate::obfuskierung::{
    CONSUMED_MESSAGE_TYPE as OBFUSKIERUNG_CONSUMED, EinlesenRowSource, KnownPersonNames,
    ObfuskierungSqliteStore, SeededObfuscator, consumer as obfuskierung_consumer,
};
use crate::outbox::OutboxHandler;
use crate::schema::{self, SchemaError, VersionMismatch};
use crate::sqlite::SqliteOutbox;

mod ack_sink;
mod history_sink;

pub use ack_sink::AckSink;
pub use history_sink::CountingHistorySink;

use application_kernel::PublishCommand;

/// Der `message_type` der Nachricht, die `obfuskieren` publiziert und die **niemand** stromabwärts
/// konsumiert (siehe Modul-Doku). Aus dem Slice-Adapter gezogen statt hier getippt — die eine Quelle
/// des Keys liegt am Slice-Ausgang.
const OBFUSCATED_MESSAGE_TYPE: &str = <ObfuscatedMessage as PublishCommand>::MESSAGE_TYPE;

/// Die Obergrenze der Broker-Runden je Phase. Sie ist **kein** Fachparameter, sondern die Zusage,
/// dass ein nicht konvergierender Lauf **laut** endet statt zu hängen: jede Runde verarbeitet den
/// Schnappschuss der offenen Nachrichten, und die Kette ist endlich (je Seite ein Fenster, je Zeile
/// eine Buchung). Bleibt trotzdem etwas offen, ist das ein Regress und keine Geduldsfrage.
const MAX_ROUNDS: usize = 4096;

/// Wo die Eingaben des Laufs liegen. Ein reiner Wert-Typ am Prozess-Rand — **öffentliche Felder**,
/// weil er nichts weiter ist als das Bündel der sechs Pfade, die die Bin-Crate von der Kommandozeile
/// bekommt; ein Konstruktor mit sechs gleichartigen Parametern wäre nur Zeremonie mit
/// Verwechslungsgefahr.
#[derive(Debug, Clone)]
pub struct RunPaths {
    /// Die SQLite-Datei, in der Outbox **und** alle vier Ergebnis-Tabellen liegen. Eine frische Datei
    /// bekommt Schema und Versionsstempel; eine passend gestempelte wird mit ihrem Bestand
    /// weiterbenutzt (at-least-once, resumierbar); eine mit abweichender Version bricht den Lauf ab
    /// ([`RunError::SchemaVersion`]).
    pub database: PathBuf,
    /// Die CSV-CAMT-v8-Datei, aus der `einlesen` seinen Batch beschafft.
    pub transactions: PathBuf,
    /// Die Fenster-Config (`window_days`).
    pub window_config: PathBuf,
    /// Die Seed-Config (`seed`).
    pub seed_config: PathBuf,
    /// Die Eigene-Konten-Config (`[[konto]]` mit `iban` + `rolle`).
    pub own_accounts_config: PathBuf,
    /// Die lokale Namensliste (`namen = [...]`) für den Freitext-Engpass von `obfuskieren` — die
    /// Namen, die `io` nicht selbst kennt (nie Empfänger, oder Vornamen/Kurzformen von Empfängern).
    /// **Pflicht**, auch wenn sie leer ist (`namen = []`): eine
    /// vergessene Liste soll den Lauf laut stoppen, statt still weniger zu scrubben.
    pub persons_config: PathBuf,
}

/// Was der Lauf getan hat: die Zahl der verarbeiteten Nachrichten **je Typ** und die Zahl der
/// Vorkommen, die die fenster-übergreifende Reichweite geliefert hat. Reine Zählwerte — nichts davon
/// ist ein Rohwert.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunReport {
    processed: BTreeMap<String, usize>,
    occurrences: usize,
}

impl RunReport {
    /// Die Zahl verarbeiteter Nachrichten je `message_type`, alphabetisch.
    #[must_use]
    pub fn processed(&self) -> &BTreeMap<String, usize> {
        &self.processed
    }

    /// Die Zahl der Vorkommen, die die fenster-übergreifende Reichweite über die ganze Zeitachse
    /// zurückgab.
    #[must_use]
    pub fn occurrences(&self) -> usize {
        self.occurrences
    }

    /// Wie viele Nachrichten der Lauf insgesamt verarbeitet hat.
    #[must_use]
    pub fn total(&self) -> usize {
        self.processed.values().sum()
    }

    fn count(&mut self, message_type: &str) {
        *self.processed.entry(message_type.to_string()).or_default() += 1;
    }
}

/// Warum ein Lauf abgebrochen ist. Jede Variante ist **redigiert**: sie trägt entweder eine statische
/// Ursache, einen statischen Dispatch-Key oder die bereits redigierte `Display` des Slice-Fehlers —
/// **nie** einen Rohwert, Freitext, eine IBAN oder den Seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// Eine der vier Configs ließ sich nicht laden.
    Config(ConfigError),
    /// Das transaktionale Rückgrat (SQLite-Datei, Tabellen) ließ sich nicht öffnen oder anlegen. Trägt
    /// die durchgereichte io-Diagnose — Zugriffs-Mechanik, kein Zeileninhalt.
    Backbone(String),
    /// Die Datenbank-Datei trägt eine **andere Schema-Version** als dieses Programm (Issue 0084) — auch
    /// eine ungestempelte Datei mit Tabellen (Version 0). Der Lauf bricht ab, **bevor** er etwas in die
    /// Datei schreibt. Trägt nur die beiden Versionsnummern, nie den Pfad.
    SchemaVersion(VersionMismatch),
    /// Zwei Consumer auf demselben Dispatch-Key — ein Verdrahtungsfehler, der nicht still verschluckt
    /// wird. Der Key ist statisch.
    DuplicateConsumer { message_type: String },
    /// Eine Nachricht ist **terminal** gescheitert. `reason` ist die redigierte `Display` des
    /// Slice-Fehlers, wie `io` sie surfacet (ADR-0012).
    Message {
        message_type: String,
        reason: String,
    },
    /// In der Outbox liegt noch ein **unerledigter Import-Auslöser einer anderen Quelle**. Ihn
    /// zuzustellen hieße, die *jetzt* übergebene Datei ein zweites Mal einzulesen — der Auslöser
    /// benennt seine Quelle zwar, aber beschafft wird sie über den Datenquellen-Port dieses Laufs
    /// (Issue 0018). Der Lauf bricht darum **vor** dem ersten Schreiben ab, statt still einen
    /// Doppelbestand anzulegen. Beide Werte sind Dateinamen, keine Pfade.
    PendingImport { pending: String, requested: String },
    /// Die Broker-Schleife kam nach der intern gedeckelten Rundenzahl nicht zur Ruhe.
    NotConverging,
    /// Das Lauf-Protokoll ließ sich nicht schreiben. Es **ist** Teil des Laufs: ein Lauf, dessen
    /// Protokoll verloren geht, gilt nicht als gelungen.
    Protocol(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(f, "Konfiguration: {error}"),
            Self::Backbone(diagnosis) => write!(f, "Rückgrat nicht verfügbar: {diagnosis}"),
            Self::SchemaVersion(mismatch) => mismatch.fmt(f),
            Self::DuplicateConsumer { message_type } => {
                write!(f, "doppelte Registrierung für {message_type}")
            }
            Self::Message {
                message_type,
                reason,
            } => write!(f, "{message_type} — {reason}"),
            Self::PendingImport { pending, requested } => write!(
                f,
                "unerledigter Import-Auslöser für {pending} — ein Lauf mit {requested} würde diese \
                 Datei doppelt einlesen; erst den offenen Lauf abschließen oder seine Nachricht \
                 entfernen"
            ),
            Self::NotConverging => f.write_str("die Broker-Schleife kam nicht zur Ruhe"),
            Self::Protocol(diagnosis) => write!(f, "Lauf-Protokoll nicht schreibbar: {diagnosis}"),
        }
    }
}

impl std::error::Error for RunError {}

impl From<ConfigError> for RunError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<SchemaError> for RunError {
    fn from(error: SchemaError) -> Self {
        match error {
            SchemaError::VersionMismatch(mismatch) => Self::SchemaVersion(mismatch),
            SchemaError::Unavailable(diagnosis) => Self::Backbone(diagnosis),
        }
    }
}

/// Fährt **einen vollständigen Pipeline-Lauf** gegen die realen `io`-Adapter und schreibt dabei das
/// redigierte Lauf-Protokoll nach `protocol`.
///
/// Gelingt der Lauf, ist die Outbox leer und der [`RunReport`] sagt, was verarbeitet wurde. Scheitert
/// **eine** Nachricht terminal, bricht der Lauf mit ihrem redigierten Fehler ab — die Nachricht bleibt
/// offen, der nächste Lauf nimmt sie wieder auf (at-least-once, resumierbar; die Versuchszählung ist
/// [Issue 0085](../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)).
pub fn run(paths: &RunPaths, protocol: &mut dyn Write) -> Result<RunReport, RunError> {
    let window_days = FensterConfig::load(&paths.window_config)?.window_days();
    let seed = SeedConfig::load(&paths.seed_config)?.seed();
    let own_accounts =
        EigeneKontenConfig::load(&paths.own_accounts_config)?.to_own_accounts_config(seed);
    let persons = PersonenConfig::load(&paths.persons_config)?.into_names();

    // Der **einzige** Weg, auf dem die Datei ihr Schema bekommt (Issue 0084): frisch → angelegt und
    // gestempelt, passend gestempelt → benutzt, jede andere Version → Abbruch, bevor etwas geschrieben
    // wird.
    let outbox = schema::open(&AppConfig::new(&paths.database))?;
    let adapters = Adapters::new(&outbox, paths, persons);
    let stores = Stores::new(&outbox);
    let occurrences = CountingHistorySink::default();
    let dispatcher = DispatcherBuilder::new(&outbox, &adapters, stores)
        .window_days(window_days)
        .seed(seed)
        .own_accounts(own_accounts)
        .occurrences(&occurrences)
        .build()?;

    let mut report = RunReport::default();

    // Phase 1 — Import. Der `partition_key` benennt die Quelle; der Slice beschafft den Batch selbst
    // über seinen Datenquellen-Port (Issue 0018), der Auslöser ist ein reiner Trigger.
    write_line(protocol, "Lauf: Import")?;
    enqueue_import(&outbox, &SourceKey::of(&paths.transactions))?;
    drain(&outbox, &dispatcher, protocol, &mut report)?;

    // Phase 2 — Analyse. **Erst jetzt**: der Anstoß liest den Initial-Anker aus dem obfuskierten
    // Bestand, den Phase 1 gerade erst aufgebaut hat (siehe Modul-Doku).
    write_line(protocol, "Lauf: Analyse")?;
    enqueue(&outbox, START_MESSAGE_TYPE, "")?;
    drain(&outbox, &dispatcher, protocol, &mut report)?;

    report.occurrences = occurrences.count();
    write_summary(protocol, &report)?;
    Ok(report)
}

/// Die realen Seams, die die Consumer nur **borrowen**, nicht besitzen — eigens gehalten, weil der
/// [`Dispatcher`] sie über die ganze Laufzeit der Broker-Schleife referenziert und sie darum nicht im
/// [`DispatcherBuilder`] selbst entstehen dürfen (der Wert würde beim Verlassen der Funktion fallen,
/// während der Dispatcher ihn noch bräuchte). [`run`] hält diesen Wert darum in seinem eigenen
/// Stack-Frame.
struct Adapters {
    transactions: CsvCamtV8Source,
    ingested_rows: EinlesenRowSource,
    person_names: KnownPersonNames,
    obfuskiert_windows: ObfuskiertWindowSource,
    analysis_windows: AuswertungsfensterSource,
    registry: EmptyCategoryRegistry,
    categorizer: UnansweredCategorizer,
}

impl Adapters {
    fn new(outbox: &SqliteOutbox, paths: &RunPaths, persons: Vec<String>) -> Self {
        Self {
            transactions: CsvCamtV8Source::new(&paths.transactions),
            ingested_rows: EinlesenRowSource::new(outbox.clone()),
            person_names: KnownPersonNames::new(outbox.clone(), persons),
            obfuskiert_windows: ObfuskiertWindowSource::new(outbox.clone()),
            analysis_windows: AuswertungsfensterSource::new(outbox.clone()),
            registry: EmptyCategoryRegistry,
            categorizer: UnansweredCategorizer,
        }
    }
}

/// Die vier Ergebnis-Stores — je einer pro Slice, alle auf **demselben** Rückgrat. Anders als
/// [`Adapters`] werden sie **besessen** in die Consumer verschoben, tragen also keine eigene
/// Lebenszeit und dürfen im [`DispatcherBuilder`] selbst geöffnet werden. Ihre Tabellen legen sie
/// **nicht** an — das hat [`schema::open`] beim Öffnen des Rückgrats schon getan. Darum muss die
/// Tabellen-Liste des Schema-Besitzers (`schema::tables`) **dieselben** Stores aufführen wie diese
/// Struktur: ein hier ergänzter Store ohne Eintrag dort hätte keine Tabelle.
struct Stores {
    einlesen: EinlesenSqliteStore,
    obfuskierung: ObfuskierungSqliteStore,
    fenstern: FensternSqliteStore,
    auswerten: AuswertenSqliteStore,
}

impl Stores {
    fn new(outbox: &SqliteOutbox) -> Self {
        Self {
            einlesen: EinlesenSqliteStore::new(outbox.clone()),
            obfuskierung: ObfuskierungSqliteStore::new(outbox.clone()),
            fenstern: FensternSqliteStore::new(outbox.clone()),
            auswerten: AuswertenSqliteStore::new(outbox.clone()),
        }
    }
}

/// Baut den [`Dispatcher`] mit allen sieben Consumern — die eine Stelle, an der die Seams
/// ([`Adapters`]), die Ergebnis-Stores ([`Stores`]) und die geladenen Config-Werte zu den realen
/// Consumern der Kette zusammenlaufen. [`run`] selbst bleibt dadurch die Zusammenfassung: Configs
/// laden, Rückgrat öffnen, Dispatcher bauen, beide Phasen fahren, Bilanz schreiben.
///
/// Die drei strukturellen Bündel — Rückgrat, Seams, Stores — verlangt `new`: ohne sie lässt sich kein
/// einziger Consumer verdrahten, und `adapters` bindet ohnehin die Lebenszeit `'a`, die der fertige
/// [`Dispatcher`] trägt. Die vier geladenen Config-Werte kommen je über eine verkettete, `self`
/// konsumierende Methode dazu — dieselbe Reihenfolge, in der [`run`] sie lädt.
///
/// Der Typparameter `C` ist der **Typestate**: er trägt die bereits gesetzten Config-Werte und wächst
/// mit jedem Schritt ([`Empty`] → [`WithWindowDays`] → [`WithSeed`] → [`WithOwnAccounts`] →
/// [`Complete`]). Jede Methode existiert nur auf ihrem Zustand, und `build` nur auf [`Complete`] —
/// einen unvollständigen Builder abzuschließen ist damit ein **Compile-Fehler** statt einer
/// Laufzeitprüfung (ADR-0005: illegale Zustände unkonstruierbar, nicht „geprüft"). Deshalb trägt
/// keines der Config-Felder ein `Option`.
struct DispatcherBuilder<'a, C> {
    outbox: &'a SqliteOutbox,
    adapters: &'a Adapters,
    stores: Stores,
    config: C,
}

/// Typestate: noch kein Config-Wert gesetzt.
struct Empty;

/// Typestate: die Fenstergröße steht.
struct WithWindowDays {
    window_days: NonZeroU32,
}

/// Typestate: Fenstergröße und Seed stehen.
struct WithSeed {
    window_days: NonZeroU32,
    seed: u64,
}

/// Typestate: dazu die tokenisierte Eigene-Konten-Config.
struct WithOwnAccounts {
    window_days: NonZeroU32,
    seed: u64,
    own_accounts: OwnAccountsConfig,
}

/// Typestate: alle vier Werte stehen — allein auf diesem Zustand gibt es `build`.
struct Complete<'a> {
    window_days: NonZeroU32,
    seed: u64,
    own_accounts: OwnAccountsConfig,
    occurrences: &'a CountingHistorySink,
}

impl<'a, C> DispatcherBuilder<'a, C> {
    /// Trägt die drei strukturellen Bündel in den nächsten Zustand, während `next` den bisherigen
    /// Config-Stand um den neuen Wert erweitert — die eine Stelle, die den Zustandsübergang vollzieht,
    /// damit die vier Setter je eine Zeile bleiben.
    fn advance<N>(self, next: impl FnOnce(C) -> N) -> DispatcherBuilder<'a, N> {
        DispatcherBuilder {
            outbox: self.outbox,
            adapters: self.adapters,
            stores: self.stores,
            config: next(self.config),
        }
    }
}

impl<'a> DispatcherBuilder<'a, Empty> {
    /// Öffnet den Builder mit den drei strukturellen Bündeln — Rückgrat, Seams, Stores —, ohne die kein
    /// Consumer entsteht.
    fn new(outbox: &'a SqliteOutbox, adapters: &'a Adapters, stores: Stores) -> Self {
        Self {
            outbox,
            adapters,
            stores,
            config: Empty,
        }
    }

    /// Die Fenstergröße für `fenstern`.
    fn window_days(self, window_days: NonZeroU32) -> DispatcherBuilder<'a, WithWindowDays> {
        self.advance(|Empty| WithWindowDays { window_days })
    }
}

impl<'a> DispatcherBuilder<'a, WithWindowDays> {
    /// Der Seed, mit dem `obfuskieren` seine Tokens ableitet.
    fn seed(self, seed: u64) -> DispatcherBuilder<'a, WithSeed> {
        self.advance(|WithWindowDays { window_days }| WithSeed { window_days, seed })
    }
}

impl<'a> DispatcherBuilder<'a, WithSeed> {
    /// Die tokenisierte Eigene-Konten-Config für `auswerten`.
    fn own_accounts(
        self,
        own_accounts: OwnAccountsConfig,
    ) -> DispatcherBuilder<'a, WithOwnAccounts> {
        self.advance(|WithSeed { window_days, seed }| WithOwnAccounts {
            window_days,
            seed,
            own_accounts,
        })
    }
}

impl<'a> DispatcherBuilder<'a, WithOwnAccounts> {
    /// Die Senke, die die fenster-übergreifenden Vorkommen zählt.
    fn occurrences(
        self,
        occurrences: &'a CountingHistorySink,
    ) -> DispatcherBuilder<'a, Complete<'a>> {
        self.advance(
            |WithOwnAccounts {
                 window_days,
                 seed,
                 own_accounts,
             }| Complete {
                window_days,
                seed,
                own_accounts,
                occurrences,
            },
        )
    }
}

impl<'a> DispatcherBuilder<'a, Complete<'a>> {
    /// Registriert alle sieben Consumer und schließt den Builder ab. Scheitern kann hier nur noch die
    /// Registrierung selbst ([`RunError::DuplicateConsumer`]) — dass alle vier Config-Werte vorliegen,
    /// hat der Typ bereits bewiesen.
    fn build(self) -> Result<Dispatcher<'a>, RunError> {
        let Complete {
            window_days,
            seed,
            own_accounts,
            occurrences,
        } = self.config;
        let outbox = self.outbox;
        let adapters = self.adapters;
        let stores = self.stores;

        let mut dispatcher = Dispatcher::new();
        register(
            &mut dispatcher,
            EINLESEN_CONSUMED,
            Box::new(einlesen_consumer(
                &adapters.transactions,
                &GermanCalendar,
                &GermanMoney,
                &IsoCurrencies,
                stores.einlesen,
            )),
        )?;
        register(
            &mut dispatcher,
            OBFUSKIERUNG_CONSUMED,
            Box::new(obfuskierung_consumer(
                &adapters.ingested_rows,
                &SeededObfuscator,
                &adapters.person_names,
                seed,
                stores.obfuskierung,
            )),
        )?;
        register(
            &mut dispatcher,
            OBFUSCATED_MESSAGE_TYPE,
            Box::new(AckSink::new(outbox.clone())),
        )?;
        register(
            &mut dispatcher,
            START_MESSAGE_TYPE,
            Box::new(start_consumer(
                &adapters.obfuskiert_windows,
                stores.fenstern.clone(),
            )),
        )?;
        register(
            &mut dispatcher,
            FENSTERN_CONSUMED,
            Box::new(fenstern_consumer(
                window_days,
                &adapters.obfuskiert_windows,
                stores.fenstern,
            )),
        )?;
        register(
            &mut dispatcher,
            AUSWERTEN_CONSUMED,
            Box::new(auswerten_consumer(
                &adapters.analysis_windows,
                &adapters.registry,
                &adapters.categorizer,
                own_accounts.clone(),
                stores.auswerten.clone(),
            )),
        )?;
        register(
            &mut dispatcher,
            CONSUMED_HISTORY_MESSAGE_TYPE,
            Box::new(history_consumer(
                &adapters.analysis_windows,
                own_accounts,
                occurrences.clone(),
                stores.auswerten,
            )),
        )?;
        Ok(dispatcher)
    }
}

/// Dreht die Broker-Schleife, bis keine offene Nachricht mehr übrig ist — nötig, weil ein Consumer
/// sich selbst fortsetzt (`fenstern` reiht je Seite die nächste Seite ein) und weil eine in dieser
/// Runde publizierte Nachricht erst die nächste Runde abholt (Schnappschuss-Semantik).
///
/// **Bricht beim ersten terminalen Fehlschlag ab**, statt ihn zu wiederholen: die gescheiterte
/// Nachricht bleibt offen, und eine halb fortgesetzte Kette weiterzudrehen hieße, auf einem
/// unvollständigen Bestand zu rechnen.
fn drain(
    outbox: &SqliteOutbox,
    dispatcher: &Dispatcher<'_>,
    protocol: &mut dyn Write,
    report: &mut RunReport,
) -> Result<(), RunError> {
    for _ in 0..MAX_ROUNDS {
        let batch = outbox.deliver_open(dispatcher).map_err(backbone)?;
        if batch.is_empty() {
            return Ok(());
        }
        for (message, outcome) in batch {
            write_line(protocol, &format!("  {}", message.message_type))?;
            outcome.map_err(|error| RunError::Message {
                message_type: message.message_type.clone(),
                reason: error.to_string(),
            })?;
            report.count(&message.message_type);
        }
    }
    Err(RunError::NotConverging)
}

/// Registriert einen Consumer und macht eine **Doppel-Registrierung** zum Fehler, statt den
/// verdrängten Consumer still fallen zu lassen.
fn register<'a>(
    dispatcher: &mut Dispatcher<'a>,
    message_type: &str,
    consumer: Box<dyn OutboxHandler + 'a>,
) -> Result<(), RunError> {
    match dispatcher.register(message_type, consumer) {
        None => Ok(()),
        Some(_displaced) => Err(RunError::DuplicateConsumer {
            message_type: message_type.to_string(),
        }),
    }
}

/// Reiht eine Wurzel-Nachricht ein und verwirft ihre ID: der Lauf zeigt auf nichts, er stößt nur an.
fn enqueue(outbox: &SqliteOutbox, message_type: &str, key: &str) -> Result<(), RunError> {
    outbox
        .enqueue(message_type, key)
        .map(|_id| ())
        .map_err(backbone)
}

/// Reiht den Import-Auslöser ein — **nachdem** geprüft ist, dass kein alter unerledigt herumliegt.
///
/// Der Grund für die Prüfung: der Auslöser **benennt** seine Quelle im `partition_key`, aber
/// **beschafft** wird sie über den Datenquellen-Port *dieses* Laufs (Issue 0018). Eine
/// liegengebliebene Nachricht liest bei der nächsten Zustellung darum die Datei, auf die das
/// *aktuelle* Kommando zeigt — nicht die, für die sie eingereiht wurde. Ohne diesen Vorab-Check
/// verwandelt sich jeder Rest eines gescheiterten Laufs still in einen zusätzlichen Import der
/// aktuellen Datei, und der Lauf meldete dafür Exit 0.
///
/// Zwei Fälle, zwei Antworten:
///
/// - **Derselbe Dateiname** — die Fortsetzung genau dieses Imports. Der offene Auslöser wird
///   wiederverwendet, statt einen zweiten danebenzulegen (der Lauf ist resümierbar, ADR-0012).
/// - **Ein anderer Dateiname** — der Lauf bricht mit [`RunError::PendingImport`] ab, **bevor** er
///   irgendetwas schreibt. Was zu tun ist, entscheidet der Mensch: den alten Lauf zu Ende fahren oder
///   seine Nachricht entfernen.
fn enqueue_import(outbox: &SqliteOutbox, key: &SourceKey) -> Result<(), RunError> {
    match outbox
        .pending_key(EINLESEN_CONSUMED)
        .map_err(backbone)?
        .map(SourceKey::from_stored)
    {
        Some(pending) if pending == *key => Ok(()),
        // Der Fehler trägt die **gerenderte** Form: er ist eine Meldung an einen Menschen, kein
        // lebender Wert — und [`RunError`] ist öffentlich, wo [`SourceKey`] es bewusst nicht ist.
        Some(pending) => Err(RunError::PendingImport {
            pending: pending.into_string(),
            requested: key.clone().into_string(),
        }),
        None => enqueue(outbox, EINLESEN_CONSUMED, key.as_str()),
    }
}

/// Der `partition_key` des Import-Auslösers: der **Dateiname** der Quelle, nie ihr Pfad.
///
/// **Warum ein eigener Typ.** „Dateiname, nie Pfad" ist keine Formalie, sondern zwei Zusagen in einer.
/// Die erste ist **Redaktion**: ein Pfad trägt die Verzeichnisstruktur der Maschine (und damit
/// womöglich einen Kontonamen) in die Outbox, wo dieses Modul sonst peinlich genau nichts hinschreibt,
/// was ein Rohwert sein könnte. Die zweite ist seit [`enqueue_import`] **Korrektheit**: an genau
/// diesem Wert erkennt ein Lauf einen fremden liegengebliebenen Auslöser, und er tut es per
/// Zeichenketten-Gleichheit. Käme an einer Stelle ein Pfad herein, wo an der anderen ein Dateiname
/// steht, verglichen sich zwei Werte, die nie gleich sein können — die Prüfung fiele **lautlos offen
/// aus** und ließe genau den stillen Doppelimport durch, gegen den sie eingezogen wurde. Ein
/// fehlschlagender Wächter, der wie ein bestandener aussieht, ist schlimmer als gar keiner.
///
/// Der Typ macht das unmöglich: [`of`](SourceKey::of) ist der einzige Weg von einem Pfad hierher, und
/// er kürzt ihn dabei. Ein `&Path` lässt sich nicht mehr versehentlich als Schlüssel durchreichen.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceKey(String);

impl SourceKey {
    /// Kürzt den Quellpfad auf seinen **Dateinamen** — der einzige Weg von einem Pfad zu einem
    /// Schlüssel. Ein Pfad ohne Dateinamen-Anteil ergibt den leeren Schlüssel; er benennt dann nichts,
    /// und der Vorab-Check in [`enqueue_import`] behandelt ihn wie jeden anderen Wert.
    fn of(transactions: &Path) -> Self {
        Self(
            transactions
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        )
    }

    /// Der Wert, wie ihn die **Outbox** trägt — abgelegt hat ihn ein früherer Lauf über
    /// [`of`](Self::of). Ohne Prüfung, wie jedes Zurücklesen: was in der Datei steht, steht dort.
    const fn from_stored(key: String) -> Self {
        Self(key)
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn into_string(self) -> String {
        self.0
    }
}

fn write_line(protocol: &mut dyn Write, line: &str) -> Result<(), RunError> {
    writeln!(protocol, "{line}").map_err(|error| RunError::Protocol(error.to_string()))
}

/// Die Schlussbilanz: die Zahl je Typ und die Gesamtzahl — beides reine Zählwerte.
fn write_summary(protocol: &mut dyn Write, report: &RunReport) -> Result<(), RunError> {
    write_line(protocol, "Lauf: verarbeitet")?;
    for (message_type, count) in report.processed() {
        write_line(protocol, &format!("  {message_type}: {count}"))?;
    }
    write_line(
        protocol,
        &format!(
            "  gesamt: {} | Vorkommen: {}",
            report.total(),
            report.occurrences()
        ),
    )
}

/// Die durchgereichte io-Diagnose eines Rückgrat-Fehlers (Zugriffs-Mechanik, nie Zeileninhalt).
fn backbone(error: rusqlite::Error) -> RunError {
    RunError::Backbone(error.to_string())
}

#[cfg(test)]
mod tests;
