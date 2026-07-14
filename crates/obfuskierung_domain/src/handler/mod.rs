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
/// besorgt io. Kann er die Buchung nicht verarbeiten, steigt er mit [`HandleError`] aus — **jeder**
/// Ausgang ist terminal (`io` spiegelt ihn über sein `Display` als Dead-Letter): **abwesend**
/// (gebrochenes Invariant, weil `einlesen` Zeile+Nachricht atomar committet und der Broker erst
/// danach zustellt) wie **ungültig** (Pflichtfeld fehlt, redigiert). Die beiden Varianten bleiben im
/// Typ getrennt, nur damit der Dead-Letter-Grund je zutrifft (opake ID vs. Feldname).
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
    /// `Ok(())` = verarbeitet; `Err(HandleError)` trennt **abwesend** von **ungültig** — beide
    /// terminal, `io` spiegelt jeden Ausgang als redigiertes Dead-Letter.
    pub fn handle(&self, command: ObfuscateBooking) -> Result<(), HandleError> {
        let aggregate = match self.source.read(command.id()) {
            BookingLookup::Found(aggregate) => *aggregate,
            BookingLookup::Absent => {
                return Err(HandleError::SourceMissing(command.id().clone()));
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

/// Warum der Handler eine Buchung nicht verarbeiten konnte — **beide** Ausgänge sind terminal; sie
/// liegen im **Typ** getrennt, damit `io` je Ausgang einen eigenen redigierten Grund spiegelt (opake
/// ID vs. statischer Feldname), nicht in einer überladenen Fehlermeldung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleError {
    /// Zur ID liegt **kein** rohes Aggregat im einlesen-Speicher vor — ein **gebrochenes Invariant**
    /// (terminal): `einlesen` committet Zeile+Nachricht atomar und der Broker stellt erst danach zu,
    /// also **liegt** die Zeile, wenn der Slice den Zeiger sieht; ihre Abwesenheit heißt korrupter
    /// Speicher / falsche ID. Trägt die opake [`BookingId`] mit (kein sensibler Wert) für den
    /// Dead-Letter-Grund. (Direkter Gegenpart zu [`BookingLookup::Absent`](crate::BookingLookup::Absent).)
    SourceMissing(BookingId),
    /// Zur ID liegt ein Aggregat vor, aber ein **Pflichtfeld fehlt/ist leer** — **terminal**
    /// (Dead-Letter). Trägt **nur** den statischen Feldnamen (redigiert, nie ein Rohwert).
    MalformedSource { field: &'static str },
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::SourceMissing(id) => {
                write!(f, "referenced booking not found in source: {}", id.as_str())
            }
            HandleError::MalformedSource { field } => {
                write!(f, "mandatory field missing in source booking: {field}")
            }
        }
    }
}

impl std::error::Error for HandleError {}
