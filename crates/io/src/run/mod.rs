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
use crate::config::{AppConfig, ConfigError, EigeneKontenConfig, FensterConfig, SeedConfig};
use crate::einlesen::{
    CONSUMED_MESSAGE_TYPE as EINLESEN_CONSUMED, CsvCamtV8Source, EinlesenSqliteStore,
    GermanCalendar, GermanMoney, IsoCurrencies, consumer as einlesen_consumer,
};
use crate::fenstern::{
    CONSUMED_MESSAGE_TYPE as FENSTERN_CONSUMED, FensternSqliteStore, ObfuskiertWindowSource,
    START_MESSAGE_TYPE, consumer as fenstern_consumer, start_consumer,
};
use crate::obfuskierung::{
    CONSUMED_MESSAGE_TYPE as OBFUSKIERUNG_CONSUMED, EinlesenRowSource, ObfuskierungSqliteStore,
    SeededObfuscator, consumer as obfuskierung_consumer,
};
use crate::outbox::OutboxHandler;
use crate::sqlite::SqliteOutbox;

mod ack_sink;
mod occurrence_sink;

pub use ack_sink::AckSink;
pub use occurrence_sink::CountingOccurrenceSink;

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
/// weil er nichts weiter ist als das Bündel der fünf Pfade, die die Bin-Crate von der Kommandozeile
/// bekommt; ein Konstruktor mit fünf gleichartigen Parametern wäre nur Zeremonie mit
/// Verwechslungsgefahr.
#[derive(Debug, Clone)]
pub struct RunPaths {
    /// Die SQLite-Datei, in der Outbox **und** alle vier Ergebnis-Tabellen liegen. Sie wird bei Bedarf
    /// angelegt; ein bestehender Bestand bleibt erhalten (at-least-once, resumierbar).
    pub database: PathBuf,
    /// Die CSV-CAMT-v8-Datei, aus der `einlesen` seinen Batch beschafft.
    pub transactions: PathBuf,
    /// Die Fenster-Config (`window_days`).
    pub window_config: PathBuf,
    /// Die Seed-Config (`seed`).
    pub seed_config: PathBuf,
    /// Die Eigene-Konten-Config (`[[konto]]` mit `iban` + `rolle`).
    pub own_accounts_config: PathBuf,
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
    /// Eine der drei Configs ließ sich nicht laden.
    Config(ConfigError),
    /// Das transaktionale Rückgrat (SQLite-Datei, Tabellen) ließ sich nicht öffnen oder anlegen. Trägt
    /// die durchgereichte io-Diagnose — Zugriffs-Mechanik, kein Zeileninhalt.
    Backbone(String),
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

    let outbox = SqliteOutbox::open(&AppConfig::new(&paths.database)).map_err(backbone)?;
    let adapters = Adapters::new(&outbox, paths);
    let stores = Stores::open(&outbox)?;
    let occurrences = CountingOccurrenceSink::default();
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
    enqueue_import(&outbox, &source_key(&paths.transactions))?;
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
    obfuskiert_windows: ObfuskiertWindowSource,
    analysis_windows: AuswertungsfensterSource,
    registry: EmptyCategoryRegistry,
    categorizer: UnansweredCategorizer,
}

impl Adapters {
    fn new(outbox: &SqliteOutbox, paths: &RunPaths) -> Self {
        Self {
            transactions: CsvCamtV8Source::new(&paths.transactions),
            ingested_rows: EinlesenRowSource::new(outbox.clone()),
            obfuskiert_windows: ObfuskiertWindowSource::new(outbox.clone()),
            analysis_windows: AuswertungsfensterSource::new(outbox.clone()),
            registry: EmptyCategoryRegistry,
            categorizer: UnansweredCategorizer,
        }
    }
}

/// Die vier Ergebnis-Stores — je einer pro Slice, alle auf **demselben** Rückgrat. Anders als
/// [`Adapters`] werden sie **besessen** in die Consumer verschoben, tragen also keine eigene
/// Lebenszeit und dürfen im [`DispatcherBuilder`] selbst geöffnet werden.
struct Stores {
    einlesen: EinlesenSqliteStore,
    obfuskierung: ObfuskierungSqliteStore,
    fenstern: FensternSqliteStore,
    auswerten: AuswertenSqliteStore,
}

impl Stores {
    fn open(outbox: &SqliteOutbox) -> Result<Self, RunError> {
        Ok(Self {
            einlesen: EinlesenSqliteStore::new(outbox.clone()).map_err(backbone)?,
            obfuskierung: ObfuskierungSqliteStore::new(outbox.clone()).map_err(backbone)?,
            fenstern: FensternSqliteStore::new(outbox.clone()).map_err(backbone)?,
            auswerten: AuswertenSqliteStore::new(outbox.clone()).map_err(backbone)?,
        })
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
    occurrences: &'a CountingOccurrenceSink,
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
        occurrences: &'a CountingOccurrenceSink,
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
                own_accounts,
                stores.auswerten.clone(),
            )),
        )?;
        register(
            &mut dispatcher,
            CONSUMED_HISTORY_MESSAGE_TYPE,
            Box::new(history_consumer(
                &adapters.analysis_windows,
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
fn enqueue_import(outbox: &SqliteOutbox, key: &str) -> Result<(), RunError> {
    match outbox.pending_key(EINLESEN_CONSUMED).map_err(backbone)? {
        Some(pending) if pending == key => Ok(()),
        Some(pending) => Err(RunError::PendingImport {
            pending,
            requested: key.to_string(),
        }),
        None => enqueue(outbox, EINLESEN_CONSUMED, key),
    }
}

/// Der `partition_key` des Import-Auslösers: der **Dateiname** der Quelle, nie ihr Pfad. Er benennt
/// den Lauf wiedererkennbar, ohne die Verzeichnisstruktur der Maschine mit in die Outbox zu schreiben
/// — und ist seit [`enqueue_import`] kein bloßes Etikett mehr, sondern das, woran ein Lauf einen
/// fremden liegengebliebenen Auslöser erkennt.
fn source_key(transactions: &Path) -> String {
    transactions
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
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
mod tests {
    use super::*;

    use crate::pipeline_test_support::TempDbPath;

    /// Die vier Ergebnis-Tabellen der Kette — je Slice die, die sein Ergebnis trägt. Dass **alle
    /// vier** nach einem Lauf gefüllt sind, ist die Aussage „die Pipeline lief durch" in
    /// nachprüfbarer Form.
    const ERGEBNIS_TABELLEN: [&str; 4] = [
        "einlesen_lines",
        "obfuskiert",
        "auswertungsfenster",
        "window_evaluations",
    ];

    /// **Rohwerte der Lauf-CSV**, die in **keiner** Ausgabe auftauchen dürfen: eine eigene IBAN, eine
    /// fremde IBAN, ein Empfängername und ein Verwendungszweck. Der Seed steht daneben als Zahl.
    const ROHWERTE: [&str; 5] = [
        "DE93100000000000000100",
        "DE10200000000000000101",
        "MUSTERMARKT SE",
        "Bezuege 06/2026",
        "4711",
    ];

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR")))
    }

    /// Die Pfade des erfundenen Lauf-Bestands: die reale CSV-Fixture und die drei realen Configs. Die
    /// **Mechanik** ist durchweg echt (SQLite-Datei, CSV-Quelle, toml-Loader) — **erfunden** sind
    /// allein die Daten.
    fn lauf_pfade(db: &TempDbPath) -> RunPaths {
        RunPaths {
            database: db.path().to_path_buf(),
            transactions: fixture("einlesen/csv/lauf.csv"),
            window_config: fixture("lauf/fenster.toml"),
            seed_config: fixture("lauf/seed.toml"),
            own_accounts_config: fixture("lauf/konten.toml"),
        }
    }

    /// Zählt über eine **eigene** Verbindung auf die Lauf-Datenbank — der Lauf ist da längst beendet,
    /// die Zählung sieht also nur committete Zeilen.
    fn zaehle(paths: &RunPaths, sql: &str) -> rusqlite::Result<i64> {
        SqliteOutbox::open(&AppConfig::new(&paths.database))?
            .with_connection(|conn| conn.query_row(sql, [], |row| row.get(0)))
    }

    /// Die Zahl noch **offener** Nachrichten — nach einem gelungenen Lauf muss sie null sein.
    fn offene_nachrichten(paths: &RunPaths) -> rusqlite::Result<i64> {
        zaehle(
            paths,
            "SELECT count(*) FROM outbox WHERE processed_on_utc IS NULL;",
        )
    }

    /// Die Zahl der Zeilen einer Ergebnis-Tabelle.
    fn zeilen(paths: &RunPaths, table: &str) -> rusqlite::Result<i64> {
        zaehle(paths, &format!("SELECT count(*) FROM {table};"))
    }

    #[expect(
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
    )]
    #[test]
    fn ein_lauf_fuellt_alle_vier_ergebnis_tabellen_und_laeuft_die_outbox_leer()
    -> rusqlite::Result<()> {
        let db = TempDbPath::new("lauf");
        let paths = lauf_pfade(&db);
        let mut protokoll = Vec::new();

        let report = run(&paths, &mut protokoll).expect("der Lauf über die erfundene CSV gelingt");

        // Kein Fake-Seam war beteiligt: echte SQLite-Datei, echte CSV-Quelle, echte Loader.
        for table in ERGEBNIS_TABELLEN {
            assert!(
                zeilen(&paths, table)? > 0,
                "die Ergebnis-Tabelle {table} muss nach dem Lauf gefüllt sein"
            );
        }
        assert_eq!(
            offene_nachrichten(&paths)?,
            0,
            "ein gelungener Lauf lässt die Outbox leer zurück"
        );

        // Die ganze Kette hat gearbeitet — jeder der sieben Dispatch-Keys mindestens einmal.
        let verarbeitet = report.processed();
        for message_type in [
            EINLESEN_CONSUMED,
            OBFUSKIERUNG_CONSUMED,
            OBFUSCATED_MESSAGE_TYPE,
            START_MESSAGE_TYPE,
            FENSTERN_CONSUMED,
            AUSWERTEN_CONSUMED,
            CONSUMED_HISTORY_MESSAGE_TYPE,
        ] {
            assert!(
                verarbeitet
                    .get(message_type)
                    .is_some_and(|count| *count > 0),
                "{message_type} muss im Lauf vorgekommen sein: {verarbeitet:?}"
            );
        }
        assert_eq!(
            verarbeitet.get(EINLESEN_CONSUMED),
            Some(&1),
            "der Import wird genau einmal angestoßen"
        );
        assert_eq!(
            verarbeitet.get(START_MESSAGE_TYPE),
            Some(&1),
            "die Analyse wird genau einmal angestoßen — nicht je Buchung"
        );
        assert_eq!(
            verarbeitet.get(OBFUSKIERUNG_CONSUMED),
            verarbeitet.get(OBFUSCATED_MESSAGE_TYPE),
            "jede obfuskierte Buchung wird auch abgeräumt, sonst bliebe sie ewig offen"
        );
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
    )]
    #[test]
    fn das_lauf_protokoll_traegt_keinen_rohwert_keinen_freitext_keine_iban_und_nicht_den_seed()
    -> rusqlite::Result<()> {
        let db = TempDbPath::new("lauf_protokoll");
        let paths = lauf_pfade(&db);
        let mut protokoll = Vec::new();

        run(&paths, &mut protokoll).expect("der Lauf gelingt");

        let text = String::from_utf8(protokoll).expect("das Protokoll ist Text");
        for rohwert in ROHWERTE {
            assert!(
                !text.contains(rohwert),
                "das Lauf-Protokoll darf `{rohwert}` nicht enthalten:\n{text}"
            );
        }
        // Positiv: es trägt die Nachrichtentypen und die Schlussbilanz — sonst bliebe die Prüfung oben
        // auch dann grün, wenn gar nichts geschrieben würde.
        assert!(text.contains(EINLESEN_CONSUMED));
        assert!(text.contains(CONSUMED_HISTORY_MESSAGE_TYPE));
        assert!(text.contains("gesamt:"));
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
    )]
    #[test]
    fn ein_terminal_gescheiterter_lauf_nennt_den_redigierten_fehler() -> rusqlite::Result<()> {
        // Terminal scheitern lassen, wo es real passiert: die Quelldatei fehlt. Der Fehler ist der
        // **nackte, redigierte** Domänen-Fehler des `einlesen`-Slice, den `io` durchreicht.
        let db = TempDbPath::new("lauf_terminal");
        let mut paths = lauf_pfade(&db);
        paths.transactions = fixture("einlesen/csv/gibt-es-nicht.csv");
        let mut protokoll = Vec::new();

        let fehler =
            run(&paths, &mut protokoll).expect_err("eine fehlende Quelldatei ist terminal");

        assert!(
            matches!(&fehler, RunError::Message { message_type, .. }
                if message_type == EINLESEN_CONSUMED),
            "der Abbruch muss die gescheiterte Nachricht benennen: {fehler:?}"
        );
        let meldung = fehler.to_string();
        assert!(
            meldung.contains("source unavailable"),
            "die durchgereichte Diagnose muss erhalten bleiben: {meldung}"
        );
        for rohwert in ROHWERTE {
            assert!(
                !meldung.contains(rohwert),
                "der Fehler darf `{rohwert}` nicht zitieren: {meldung}"
            );
        }
        // Die Nachricht bleibt **offen** — der nächste Lauf nimmt sie wieder auf (ADR-0012/Issue 0085).
        assert_eq!(offene_nachrichten(&paths)?, 1);
        Ok(())
    }

    #[test]
    fn eine_fehlende_config_beendet_den_lauf_bevor_irgendetwas_laeuft() {
        let db = TempDbPath::new("lauf_config");
        let mut paths = lauf_pfade(&db);
        paths.own_accounts_config = fixture("lauf/gibt-es-nicht.toml");
        let mut protokoll = Vec::new();

        let fehler = run(&paths, &mut protokoll).expect_err("fehlende Config");

        assert_eq!(
            fehler,
            RunError::Config(ConfigError::Unreadable(
                crate::config::ConfigKind::EigeneKonten
            ))
        );
        assert!(
            protokoll.is_empty(),
            "vor der geladenen Config wird nichts protokolliert — und nichts angestoßen"
        );
    }

    #[expect(
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
    )]
    #[test]
    fn ein_zweiter_lauf_auf_derselben_datei_stoesst_den_import_erneut_an() -> rusqlite::Result<()> {
        // Der Lauf ist **resumierbar**, nicht idempotent: er reiht seine Auslöser jedes Mal neu ein.
        // Diese Zusage festzuhalten ist wichtiger als sie zu verstecken — wer zweimal importiert, hat
        // den Bestand zweimal. Die Entdopplung ist ein eigenes Thema (Schema/Versionsstempel,
        // Issue 0084), kein stiller Nebeneffekt hier.
        let db = TempDbPath::new("lauf_zweimal");
        let paths = lauf_pfade(&db);
        let mut protokoll = Vec::new();

        run(&paths, &mut protokoll).expect("erster Lauf");
        let nach_dem_ersten = zeilen(&paths, "einlesen_lines")?;
        run(&paths, &mut protokoll).expect("zweiter Lauf");

        assert_eq!(
            zeilen(&paths, "einlesen_lines")?,
            nach_dem_ersten * 2,
            "zwei Läufe über dieselbe Datei importieren zweimal"
        );
        assert_eq!(offene_nachrichten(&paths)?, 0, "und beide laufen leer");
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens; der Result-Rückgabewert                   trägt allein die DB-Abfragen des Tests, nicht seinen Ausgang"
    )]
    #[test]
    fn ein_offener_ausloeser_einer_anderen_quelle_bricht_den_lauf_ab() -> rusqlite::Result<()> {
        // Der Fall, der vorher **still** danebenging: ein gescheiterter Lauf lässt seinen Auslöser
        // offen liegen. Da der Slice seine Quelle über den Datenquellen-Port *dieses* Laufs beschafft
        // (Issue 0018) und nicht über den `partition_key`, las die liegengebliebene Nachricht beim
        // nächsten Mal die **neue** Datei — der Lauf importierte sie zweimal und meldete Exit 0.
        let db = TempDbPath::new("lauf_fremder_rest");
        let mut kaputt = lauf_pfade(&db);
        kaputt.transactions = fixture("einlesen/csv/pflichtfeld_fehlt.csv");
        let mut protokoll = Vec::new();

        run(&kaputt, &mut protokoll).expect_err("ein fehlendes Pflichtfeld ist terminal");
        assert_eq!(
            offene_nachrichten(&kaputt)?,
            1,
            "der gescheiterte Auslöser bleibt offen (ADR-0012)"
        );

        let gut = lauf_pfade(&db);
        let fehler = run(&gut, &mut protokoll).expect_err("der fremde Rest muss den Lauf stoppen");

        assert!(
            matches!(&fehler, RunError::PendingImport { pending, requested }
                if pending == "pflichtfeld_fehlt.csv" && requested == "lauf.csv"),
            "der Abbruch muss beide Quellen benennen: {fehler:?}"
        );
        assert_eq!(
            zeilen(&gut, "einlesen_lines")?,
            0,
            "und zwar **bevor** irgendetwas geschrieben wurde"
        );
        Ok(())
    }
}
