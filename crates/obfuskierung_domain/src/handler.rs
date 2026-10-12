use crate::ports::{
    BookingPublisher, KnownPersonsSource, ObfuscatedRepository, Obfuscator, RawBookingSource,
};
use crate::{ObfuscateBooking, ObfuscationError, Seed};

/// Orchestriert die Obfuskierung **einer** Buchung — er **lebt in der Domäne** und hängt **nur** an
/// den von ihr verlangten Ports; die Verdrahtung an die Außenwelt (io/Fakes) besorgt der Adapter über
/// Port-Bridges. Den injizierten [`Seed`] hält er als Wert.
///
/// **Ablauf je Kommando:**
/// 1. das rohe Aggregat **per ID lesen** ([`RawBookingSource`]),
/// 2. die **bekannten Personen-Namen** holen ([`KnownPersonsSource`]) und das Aggregat seed-basiert
///    **obfuskieren** ([`Obfuscator`]) — der Freitext wird dabei gegen diese Namen gescrubbt,
/// 3. das obfuskierte Aggregat ins **Repository ablegen** ([`ObfuscatedRepository`], das die **neue
///    ID** vergibt),
/// 4. **diese neue ID** als Zeiger **publizieren** ([`BookingPublisher`]).
///
/// Der Handler kennt **keine** Transaktion: Ablage und Publish *stagen* nur, den atomaren Commit
/// besorgt io. Lesen (Buchung und Namensliste), Ablegen und Publizieren sind je fehlbar und terminal;
/// ihr [`ObfuscationError`] propagiert per `?`.
pub struct ObfuscationHandler<'a> {
    seed: Seed,
    source: &'a dyn RawBookingSource,
    obfuscator: &'a dyn Obfuscator,
    persons: &'a dyn KnownPersonsSource,
    repository: &'a dyn ObfuscatedRepository,
    publisher: &'a dyn BookingPublisher,
}

impl<'a> ObfuscationHandler<'a> {
    pub fn new(
        seed: Seed,
        source: &'a dyn RawBookingSource,
        obfuscator: &'a dyn Obfuscator,
        persons: &'a dyn KnownPersonsSource,
        repository: &'a dyn ObfuscatedRepository,
        publisher: &'a dyn BookingPublisher,
    ) -> Self {
        Self {
            seed,
            source,
            obfuscator,
            persons,
            repository,
            publisher,
        }
    }

    /// Verarbeitet das Kommando: lesen → obfuskieren → (staged) ablegen → neue ID publizieren.
    /// `Ok(())` = verarbeitet; `Err(ObfuscationError)` = terminaler Fehler (der Adapter reicht ihn an io).
    pub fn handle(&self, command: ObfuscateBooking) -> Result<(), ObfuscationError> {
        let booking = self.source.read(command.id())?;
        let persons = self.persons.known_persons()?;
        let obfuscated = booking.obfuscate(self.seed, self.obfuscator, &persons);
        let id = self.repository.save(&obfuscated)?;
        self.publisher.publish(&id)?;
        Ok(())
    }
}
