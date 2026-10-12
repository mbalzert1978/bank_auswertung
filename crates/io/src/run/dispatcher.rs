//! Der **Bau des [`Dispatcher`]** für einen Lauf: die eine Stelle, an der Seams, Ergebnis-Stores und
//! geladene Config-Werte zu den realen Consumern der Kette zusammenlaufen — aus `run/mod.rs`
//! ausgelagert, unverändert.

use std::num::NonZeroU32;

use auswerten_adapter::{OwnAccountsConfig, SavingsThresholdConfig};

use super::{
    AUSWERTEN_CONSUMED, AckSink, Adapters, CONSUMED_HISTORY_MESSAGE_TYPE, EINLESEN_CONSUMED,
    FENSTERN_CONSUMED, GermanCalendar, GermanMoney, IsoCurrencies, OBFUSCATED_MESSAGE_TYPE,
    OBFUSKIERUNG_CONSUMED, RecordingHistorySink, RunError, START_MESSAGE_TYPE, SeededObfuscator,
    Stores, auswerten_consumer, einlesen_consumer, fenstern_consumer, history_consumer,
    obfuskierung_consumer, start_consumer,
};
use crate::Dispatcher;
use crate::outbox::OutboxHandler;
use crate::sqlite::SqliteOutbox;

/// Baut den [`Dispatcher`] mit allen sieben Consumern — die eine Stelle, an der die Seams
/// ([`Adapters`]), die Ergebnis-Stores ([`Stores`]) und die geladenen Config-Werte zu den realen
/// Consumern der Kette zusammenlaufen. [`run`] selbst bleibt dadurch die Zusammenfassung: Configs
/// laden, Rückgrat öffnen, Dispatcher bauen, beide Phasen fahren, Bilanz schreiben.
///
/// Die drei strukturellen Bündel — Rückgrat, Seams, Stores — verlangt `new`: ohne sie lässt sich kein
/// einziger Consumer verdrahten, und `adapters` bindet ohnehin die Lebenszeit `'a`, die der fertige
/// [`Dispatcher`] trägt. Die geladenen Config-Werte und die Senke kommen je über eine verkettete, `self`
/// konsumierende Methode dazu — dieselbe Reihenfolge, in der [`run`] sie lädt.
///
/// Der Typparameter `C` ist der **Typestate**: er trägt die bereits gesetzten Config-Werte und wächst
/// mit jedem Schritt ([`Empty`] → [`WithWindowDays`] → [`WithSeed`] → [`WithOwnAccounts`] →
/// [`WithSavingsThreshold`] → [`Complete`]). Jede Methode existiert nur auf ihrem Zustand, und `build` nur auf [`Complete`] —
/// einen unvollständigen Builder abzuschließen ist damit ein **Compile-Fehler** statt einer
/// Laufzeitprüfung (ADR-0005: illegale Zustände unkonstruierbar, nicht „geprüft"). Deshalb trägt
/// keines der Config-Felder ein `Option`.
pub(super) struct DispatcherBuilder<'a, C> {
    outbox: &'a SqliteOutbox,
    adapters: &'a Adapters,
    stores: Stores,
    config: C,
}

/// Typestate: noch kein Config-Wert gesetzt.
pub(super) struct Empty;

/// Typestate: die Fenstergröße steht.
pub(super) struct WithWindowDays {
    window_days: NonZeroU32,
}

/// Typestate: Fenstergröße und Seed stehen.
pub(super) struct WithSeed {
    window_days: NonZeroU32,
    seed: u64,
}

/// Typestate: dazu die tokenisierte Eigene-Konten-Config.
pub(super) struct WithOwnAccounts {
    window_days: NonZeroU32,
    seed: u64,
    own_accounts: OwnAccountsConfig,
}

/// Typestate: dazu die Spar-Schwelle.
pub(super) struct WithSavingsThreshold {
    window_days: NonZeroU32,
    seed: u64,
    own_accounts: OwnAccountsConfig,
    savings_threshold: SavingsThresholdConfig,
}

/// Typestate: alle Werte stehen — allein auf diesem Zustand gibt es `build`.
pub(super) struct Complete<'a> {
    window_days: NonZeroU32,
    seed: u64,
    own_accounts: OwnAccountsConfig,
    savings_threshold: SavingsThresholdConfig,
    history: &'a RecordingHistorySink,
}

impl<'a, C> DispatcherBuilder<'a, C> {
    /// Trägt die drei strukturellen Bündel in den nächsten Zustand, während `next` den bisherigen
    /// Config-Stand um den neuen Wert erweitert — die eine Stelle, die den Zustandsübergang vollzieht,
    /// damit die Setter je eine Zeile bleiben.
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
    pub(super) fn new(outbox: &'a SqliteOutbox, adapters: &'a Adapters, stores: Stores) -> Self {
        Self {
            outbox,
            adapters,
            stores,
            config: Empty,
        }
    }

    /// Die Fenstergröße für `fenstern`.
    pub(super) fn window_days(
        self,
        window_days: NonZeroU32,
    ) -> DispatcherBuilder<'a, WithWindowDays> {
        self.advance(|Empty| WithWindowDays { window_days })
    }
}

impl<'a> DispatcherBuilder<'a, WithWindowDays> {
    /// Der Seed, mit dem `obfuskieren` seine Tokens ableitet.
    pub(super) fn seed(self, seed: u64) -> DispatcherBuilder<'a, WithSeed> {
        self.advance(|WithWindowDays { window_days }| WithSeed { window_days, seed })
    }
}

impl<'a> DispatcherBuilder<'a, WithSeed> {
    /// Die tokenisierte Eigene-Konten-Config für `auswerten`.
    pub(super) fn own_accounts(
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
    /// Die Spar-Schwelle für die fenster-übergreifende Reichweite von `auswerten`.
    pub(super) fn savings_threshold(
        self,
        savings_threshold: SavingsThresholdConfig,
    ) -> DispatcherBuilder<'a, WithSavingsThreshold> {
        self.advance(
            |WithOwnAccounts {
                 window_days,
                 seed,
                 own_accounts,
             }| WithSavingsThreshold {
                window_days,
                seed,
                own_accounts,
                savings_threshold,
            },
        )
    }
}

impl<'a> DispatcherBuilder<'a, WithSavingsThreshold> {
    /// Der Abnehmer, der das Ergebnis der fenster-übergreifenden Reichweite für den Bericht festhält.
    pub(super) fn history_sink(
        self,
        history: &'a RecordingHistorySink,
    ) -> DispatcherBuilder<'a, Complete<'a>> {
        self.advance(
            |WithSavingsThreshold {
                 window_days,
                 seed,
                 own_accounts,
                 savings_threshold,
             }| Complete {
                window_days,
                seed,
                own_accounts,
                savings_threshold,
                history,
            },
        )
    }
}

impl<'a> DispatcherBuilder<'a, Complete<'a>> {
    /// Registriert alle sieben Consumer und schließt den Builder ab. Scheitern kann hier nur noch die
    /// Registrierung selbst ([`RunError::DuplicateConsumer`]) — dass alle Werte vorliegen,
    /// hat der Typ bereits bewiesen.
    pub(super) fn build(self) -> Result<Dispatcher<'a>, RunError> {
        let Complete {
            window_days,
            seed,
            own_accounts,
            savings_threshold,
            history,
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
                savings_threshold,
                history.clone(),
                stores.auswerten,
            )),
        )?;
        Ok(dispatcher)
    }
}

/// Registriert einen Consumer und macht eine **Doppel-Registrierung** zum Fehler, statt den
/// verdrängten Consumer still fallen zu lassen.
pub(super) fn register<'a>(
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
