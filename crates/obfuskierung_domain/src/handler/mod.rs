use std::fmt;

use crate::ports::{
    BookingLookup, BookingPublisher, ObfuscatedRepository, Obfuscator, RawBookingSource,
};
use crate::{BookingId, ObfuscateBooking, Seed};

/// Orchestriert die Obfuskierung **einer** Buchung — er **lebt in der Domäne** und hängt **nur** an
/// den von ihr verlangten Ports (Vorbild: der `ReadHandler` des Einlese-Slice, der an
/// `Calendar`/`Money`/`Store`/`EventSink` hängt). Den injizierten [`Seed`] hält er als Wert; die
/// Verdrahtung an die Außenwelt (io/Fakes) besorgt der Adapter über Port-Bridges.
///
/// **Ablauf je Kommando (Vorbild-Pseudocode):**
/// 1. das rohe Aggregat **per ID aus dem einlesen-Speicher lesen** ([`RawBookingSource`]),
/// 2. es seed-basiert **obfuskieren** ([`Obfuscator`]),
/// 3. das obfuskierte Aggregat ins **Repository ablegen** ([`ObfuscatedRepository`]),
/// 4. seine **ID als Zeiger publizieren** ([`BookingPublisher`]).
///
/// Der Handler kennt **keine** Transaktion: Ablage und Publish *stagen* nur, den atomaren Commit
/// besorgt io. Kann er die Buchung nicht verarbeiten, steigt er mit [`HandleError`] aus — **abwesend**
/// (noch kein Aggregat) mappt der Adapter transient (Retry), **ungültig** (Pflichtfeld fehlt) terminal
/// (Dead-Letter, redigiert).
pub struct ObfuscationHandler<'a> {
    seed: Seed,
    source: &'a dyn RawBookingSource,
    obfuscator: &'a dyn Obfuscator,
    repository: &'a dyn ObfuscatedRepository,
    publisher: &'a dyn BookingPublisher,
}

impl<'a> ObfuscationHandler<'a> {
    pub fn new(
        seed: Seed,
        source: &'a dyn RawBookingSource,
        obfuscator: &'a dyn Obfuscator,
        repository: &'a dyn ObfuscatedRepository,
        publisher: &'a dyn BookingPublisher,
    ) -> Self {
        Self {
            seed,
            source,
            obfuscator,
            repository,
            publisher,
        }
    }

    /// Verarbeitet das Kommando: lesen → obfuskieren → (staged) ablegen + Zeiger publizieren.
    /// `Ok(())` = verarbeitet; `Err(HandleError)` trennt **abwesend** (transient) von **ungültig**
    /// (terminal, redigiert) — der Adapter mappt beide auf die passende Outbox-Fehlerklasse.
    pub fn handle(&self, command: ObfuscateBooking) -> Result<(), HandleError> {
        let aggregate = match self.source.read(command.id()) {
            BookingLookup::Found(aggregate) => *aggregate,
            BookingLookup::Absent => {
                return Err(HandleError::NotYetAvailable(command.id().clone()));
            }
            BookingLookup::Malformed { field } => {
                return Err(HandleError::MalformedSource { field });
            }
        };
        let obfuscated = aggregate.obfuscate(self.seed, self.obfuscator);
        self.repository.save(&obfuscated);
        self.publisher.publish(obfuscated.id());
        Ok(())
    }
}

/// Warum der Handler eine Buchung nicht verarbeiten konnte — die beiden Ausgänge liegen im **Typ**,
/// nicht in einer überladenen Fehlermeldung, damit der Adapter transient von terminal sauber trennt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleError {
    /// Zur ID liegt (noch) **kein** rohes Aggregat im einlesen-Speicher vor — **transient** (die
    /// einlesen-Schreibung kann nachhängen). Trägt die opake [`BookingId`] mit (kein sensibler Wert)
    /// für Diagnose/Wiedervorlage; der Adapter mappt sie auf erneute Zustellung (Retry).
    NotYetAvailable(BookingId),
    /// Zur ID liegt ein Aggregat vor, aber ein **Pflichtfeld fehlt/ist leer** — **terminal**
    /// (Dead-Letter). Trägt **nur** den statischen Feldnamen (redigiert, nie ein Rohwert).
    MalformedSource { field: &'static str },
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::NotYetAvailable(id) => {
                write!(f, "referenced booking not found in source: {}", id.as_str())
            }
            HandleError::MalformedSource { field } => {
                write!(f, "mandatory field missing in source booking: {field}")
            }
        }
    }
}

impl std::error::Error for HandleError {}
