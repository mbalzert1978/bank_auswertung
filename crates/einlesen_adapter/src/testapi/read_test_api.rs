use application_kernel::{CommandDecoder, Handler};
use einlesen_domain::ReadError;

use super::fakes::{
    FakeCalendarSeam, FakeCurrencySeam, FakeMoneySeam, FakePublisher, FakeSourceSeam, FakeStoreSeam,
};
use crate::handler::ReadingHandler;
use crate::models::ReadCommandDecoder;
use crate::models::{Line, RawRecord};

/// Fluent-TestApi des einlesen-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md),
/// [ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)): die
/// **Eingangsfläche spricht nur DTOs** (`RawRecord` in der Quelle arrangieren), `run` fährt dann
/// die **echte** Kette über den `application_kernel`-Vertrag — [`ReadCommandDecoder`] (Envelope-
/// Strings → Kommando, Shape Y) → [`ReadingHandler`] (Quell-/Kalender-/Geld-/Speicher-/Publisher-
/// Bridge + domänen-interner Handler) gegen öffentliche Fakes. **Keine Outbox-Mechanik mehr:** der
/// Slice ist eine reine Pipeline; die `OutboxConsumer`/`UnitOfWork`-Taktik prüft der
/// io-Integrationstest, nicht dieser Spec.
///
/// Die Nachricht ist nur der **Auslöser** — den Batch beschafft der Handler über den Quell-Seam.
/// Beobachtet wird zweifach: die veröffentlichten „Eingelesen"-Commands (Fan-out) **und** die
/// abgelegten Aggregate am Speicher-Seam. `run` gibt den **nackten Domänen-[`ReadError`]** zurück
/// (kein `OutboxError`): eine nicht beschaffbare Quelle, eine ungültige Zeile oder eine
/// gescheiterte Ablage/Veröffentlichung lässt ihn **terminal** scheitern.
pub struct ReadTestApi {
    source_seam: FakeSourceSeam,
    calendar_seam: FakeCalendarSeam,
    money_seam: FakeMoneySeam,
    currency_seam: FakeCurrencySeam,
    store_seam: FakeStoreSeam,
    publisher: FakePublisher,
}

impl ReadTestApi {
    /// Neue, leere TestApi.
    #[must_use]
    pub fn new() -> Self {
        Self {
            source_seam: FakeSourceSeam::default(),
            calendar_seam: FakeCalendarSeam,
            money_seam: FakeMoneySeam,
            currency_seam: FakeCurrencySeam,
            store_seam: FakeStoreSeam::default(),
            publisher: FakePublisher::default(),
        }
    }

    /// Arrangiert eine Rohzeile (DTO) in der Quelle hinter dem Datenquellen-Port
    /// (in Lesereihenfolge) — nicht in der Nachricht; die trägt keinen Batch mehr.
    #[must_use]
    pub fn with_line(mut self, record: RawRecord) -> Self {
        self.source_seam.add_record(record);
        self
    }

    /// Arrangiert den **Fehlschlag der Beschaffung**: der Quell-Seam scheitert terminal mit der
    /// gegebenen io-Diagnose (statt Zeilen zu liefern) — `run` endet im terminalen Fehler, dessen
    /// Grund die Diagnose trägt.
    #[must_use]
    pub fn with_unavailable_source(mut self, message: &str) -> Self {
        self.source_seam.fail_with(message);
        self
    }

    /// Arrangiert den **Fehlschlag der Ablage**: der Speicher-Seam scheitert terminal mit der
    /// gegebenen io-Diagnose — `run` endet im terminalen Fehler, dessen Grund die Diagnose trägt.
    #[must_use]
    pub fn with_failing_store(mut self, message: &str) -> Self {
        self.store_seam.fail_with(message);
        self
    }

    /// Arrangiert den **Fehlschlag der Veröffentlichung**: der Publish-Seam scheitert terminal mit
    /// der gegebenen io-Diagnose — `run` endet im terminalen Fehler, dessen Grund die Diagnose trägt.
    #[must_use]
    pub fn with_failing_publish(mut self, message: &str) -> Self {
        self.publisher.fail_with(message);
        self
    }

    /// Fährt die echte Kette (Decoder → Handler) für einen `ReadTransactions`-Auslöser der
    /// gegebenen Quelle und gibt das Handler-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` =
    /// terminaler [`ReadError`]). Die Envelope ist **inhaltslos** — die Quelle reist nur als
    /// `partition_key`.
    pub fn run(&self, source: &str) -> Result<(), ReadError> {
        // Der einlesen-Auslöser trägt keine Nutzlast und ist stets dekodierbar; ein `None` ist
        // hier ein No-op statt eines Panics — die TestApi ist so die eine Stelle, die auf einen
        // Decode nicht paniken kann.
        let Some(command) = ReadCommandDecoder::decode(source, "") else {
            return Ok(());
        };
        let handler = ReadingHandler::new(
            &self.source_seam,
            &self.calendar_seam,
            &self.money_seam,
            &self.currency_seam,
            &self.store_seam,
            &self.publisher,
        );
        handler.handle(command)
    }

    /// Die veröffentlichten Publish-Command-Types (z. B. `Ingested`) in Reihenfolge.
    pub fn published_message_types(&self) -> Vec<String> {
        self.publisher.message_types()
    }

    /// Die Partition-Keys der veröffentlichten „Eingelesen"-Kommandos in Lesereihenfolge — je
    /// Nachricht die **Aggregat-ID** (der Zeiger auf die abgelegte Roh-Zeile). Reiner
    /// Datenrückgeber: der Spec vergleicht sie per `assert_eq!` mit
    /// [`stored_ids`](Self::stored_ids) (Tabelle = Wahrheit, Message = Pointer) — die TestApi
    /// assertiert nicht.
    pub fn published_partition_keys(&self) -> Vec<String> {
        self.publisher.partition_keys()
    }

    /// Die Schlüssel, unter denen die Roh-Zeilen am Speicher-Seam abgelegt wurden, in
    /// Lesereihenfolge — je Zeile ein **eigener**, verschiedener Schlüssel. Der Spec vergleicht
    /// sie per `assert_eq!` mit [`published_partition_keys`](Self::published_partition_keys):
    /// Gleichheit belegt, dass jeder Zeiger genau eine abgelegte Zeile deckt (und, weil die
    /// Schlüssel je Zeile verschieden sind, zugleich die paarweise Verschiedenheit der Zeiger).
    /// Der Spec sieht so nur die Deckung zweier Datenrückgaben, nie einen konkreten ID-Wert als
    /// Literal.
    pub fn stored_ids(&self) -> Vec<String> {
        self.store_seam.stored_ids()
    }

    /// Die `ordering_account`-Werte der **gestageten** Aggregate am Speicher-Seam, in
    /// Reihenfolge — so prüft ein Spec, dass je gültiger Zeile persistiert wurde.
    pub fn stored_ordering_accounts(&self) -> Vec<String> {
        self.store_seam.ordering_accounts()
    }

    /// Die **gestageten** [`Line`]-DTOs am Speicher-Seam, in Lesereihenfolge — so prüft ein
    /// Spec die Feld-Treue der Projektion (alle Felder inkl. Währung und Optional-`none`) am
    /// realen Durchlauf, am konkreten DTO statt per generischem Serialisieren.
    pub fn stored_lines(&self) -> Vec<Line> {
        self.store_seam.lines()
    }
}

impl Default for ReadTestApi {
    fn default() -> Self {
        Self::new()
    }
}
