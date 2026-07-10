use einlesen_domain::ReadHandler;
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use super::fakes::{
    FakeCalendarSeam, FakeMoneySeam, FakeOutboxPublisher, FakeSourceSeam, FakeStoreSeam,
};
use crate::bridges::{
    CalendarBridge, ConsumeBridge, IngestedPublisher, MoneyBridge, SourceBridge, StoreBridge,
};
use crate::commands::read_transactions_message::{self, ReadTransactionsMessage};
use crate::dto::{Line, RawRecord};

/// Fluent-TestApi des einlesen-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// die **Eingangsfläche spricht nur DTOs** (`RawRecord` in der Quelle arrangieren), `run` fährt
/// dann eine **echte** Kette Consume-Bridge → Quell-Bridge (Beschaffung über den
/// Datenquellen-Port) → Kalender-/Geld-/Speicher-Bridge + domänen-interner [`ReadHandler`] →
/// Publisher-Bridge gegen öffentliche Fakes (Vorbild `DownloaderTestApi`: echte Bridges gegen
/// Fakes der public Abstractions, nie gegen interne Domänen-Ports direkt). Die Nachricht ist nur
/// der **Auslöser** — den Batch beschafft der Handler über den Quell-Seam. Beobachtet wird
/// zweifach: die veröffentlichten „Eingelesen"-Commands (Fan-out) **und** die gestageten
/// Aggregate am Speicher-Seam. Eine nicht beschaffbare Quelle oder eine ungültige Zeile lässt
/// `run` **terminal** mit `OutboxError::Warning` scheitern (io dead-lettert dann); nichts wird
/// veröffentlicht/gestaged.
pub struct ReadTestApi {
    source_seam: FakeSourceSeam,
    calendar_seam: FakeCalendarSeam,
    money_seam: FakeMoneySeam,
    store_seam: FakeStoreSeam,
    outbox: FakeOutboxPublisher,
}

impl ReadTestApi {
    /// Neue, leere TestApi.
    pub fn new() -> Self {
        Self {
            source_seam: FakeSourceSeam::default(),
            calendar_seam: FakeCalendarSeam,
            money_seam: FakeMoneySeam,
            store_seam: FakeStoreSeam::default(),
            outbox: FakeOutboxPublisher::default(),
        }
    }

    /// Arrangiert eine Rohzeile (DTO) in der Quelle hinter dem Datenquellen-Port
    /// (in Lesereihenfolge) — nicht in der Nachricht; die trägt keinen Batch mehr.
    pub fn with_line(mut self, record: RawRecord) -> Self {
        self.source_seam.add_record(record);
        self
    }

    /// Arrangiert den **Fehlschlag der Beschaffung**: der Quell-Seam scheitert terminal mit der
    /// gegebenen io-Diagnose (statt Zeilen zu liefern) — `run` endet im Dead-Letter, dessen
    /// Grund die Diagnose trägt.
    pub fn with_unavailable_source(mut self, message: &str) -> Self {
        self.source_seam.fail_with(message);
        self
    }

    /// Fährt die echte Bridge-Kette für eine `ReadTransactions`-Nachricht der gegebenen Quelle
    /// und gibt das Outbox-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` = Dead-Letter).
    /// Die Payload ist **feldlos** (`{}`) — die Quelle reist nur als `partition_key` des
    /// Envelopes (ADR-0008-Envelope-Konvention).
    pub fn run(&self, source: &str) -> Result<(), OutboxError> {
        let content = serde_json::to_string(&ReadTransactionsMessage {}).map_err(|error| {
            OutboxError::Critical {
                message: format!("ReadTransactions not serializable: {error}"),
                source: read_transactions_message::MESSAGE_TYPE.to_string(),
            }
        })?;
        let line_source = SourceBridge {
            seam: &self.source_seam,
        };
        let calendar = CalendarBridge {
            seam: &self.calendar_seam,
        };
        let money = MoneyBridge {
            seam: &self.money_seam,
        };
        let store = StoreBridge {
            seam: &self.store_seam,
        };
        let publisher = IngestedPublisher {
            outbox: &self.outbox,
        };
        let handler = ReadHandler::new(&line_source, &calendar, &money, &store, &publisher);
        let bridge = ConsumeBridge { handler };
        let message = OutboxMessage {
            id: "test-message".to_string(),
            message_type: read_transactions_message::MESSAGE_TYPE.to_string(),
            partition_key: source.to_string(),
            content,
            occurred_on_utc: 0,
            attempt: 0,
        };
        bridge.handle(message)
    }

    /// Die veröffentlichten Outbox-Message-Types (z. B. `Ingested`) in Reihenfolge.
    pub fn published_message_types(&self) -> Vec<String> {
        self.outbox.message_types()
    }

    /// Die Partition-Keys der veröffentlichten „Eingelesen"-Kommandos in Lesereihenfolge — je
    /// Nachricht die **Aggregat-ID** (der Zeiger auf die abgelegte Roh-Zeile). Reiner
    /// Datenrückgeber: der Spec vergleicht sie per `assert_eq!` mit
    /// [`stored_ids`](Self::stored_ids) (Tabelle = Wahrheit, Message = Pointer) — die TestApi
    /// assertiert nicht.
    pub fn published_partition_keys(&self) -> Vec<String> {
        self.outbox.partition_keys()
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
