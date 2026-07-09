use std::fmt;

use crate::ports::{BookingPublisher, ObfuscatedRepository, Obfuscator, RawBookingSource};
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
/// besorgt io. Ist zur ID (noch) kein Aggregat da, steigt er mit [`UnknownBooking`] aus (der Adapter
/// mappt das auf einen transienten Outbox-Fehler → erneute Zustellung).
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
    /// `Ok(())` = verarbeitet, `Err(UnknownBooking)` = zur ID liegt (noch) kein Aggregat vor.
    pub fn handle(&self, command: ObfuscateBooking) -> Result<(), UnknownBooking> {
        let aggregate = self
            .source
            .read(command.id())
            .ok_or_else(|| UnknownBooking::new(command.id().clone()))?;
        let obfuscated = aggregate.obfuscate(self.seed, self.obfuscator);
        self.repository.save(&obfuscated);
        self.publisher.publish(obfuscated.id());
        Ok(())
    }
}

/// Zur referenzierten ID liegt (noch) kein rohes Aggregat im einlesen-Speicher vor. Trägt die
/// [`BookingId`] mit — sie ist ein opaker Identifikator ohne sensible Daten und gehört in den
/// Dead-Letter-Grund (Diagnose/Wiedervorlage); der Adapter mappt den Fehler auf einen transienten
/// Outbox-Fehler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownBooking {
    id: BookingId,
}

impl UnknownBooking {
    fn new(id: BookingId) -> Self {
        Self { id }
    }
}

impl fmt::Display for UnknownBooking {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "referenced booking not found in source: {}",
            self.id.as_str()
        )
    }
}

impl std::error::Error for UnknownBooking {}
