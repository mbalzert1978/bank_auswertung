use einlesen_domain::ReadHandler;
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use super::fakes::{FakeCalendarSeam, FakeMoneySeam, FakeOutboxPublisher, FakeStoreSeam};
use crate::bridges::{CalendarBridge, ConsumeBridge, IngestedPublisher, MoneyBridge, StoreBridge};
use crate::commands::read_transactions_message::{self, ReadTransactionsMessage};
use crate::dto::RawRecord;

/// Fluent-TestApi des einlesen-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// die **Eingangsfläche spricht nur DTOs** (`RawRecord` arrangieren), `run` fährt dann eine
/// **echte** Kette Consume-Bridge → Kalender-/Geld-/Speicher-Bridge + domänen-interner
/// [`ReadHandler`] → Publisher-Bridge gegen öffentliche Fakes (Vorbild `DownloaderTestApi`:
/// echte Bridges gegen Fakes der public Abstractions, nie gegen interne Domänen-Ports direkt).
/// Beobachtet wird zweifach: die veröffentlichten „Eingelesen"-Commands (Fan-out) **und** die
/// gestageten Aggregate am Speicher-Seam. Eine ungültige Zeile lässt `run` **terminal** mit
/// `OutboxError::Warning` scheitern (io dead-lettert dann); nichts wird veröffentlicht/gestaged.
pub struct ReadTestApi {
    lines: Vec<RawRecord>,
    calendar_seam: FakeCalendarSeam,
    money_seam: FakeMoneySeam,
    store_seam: FakeStoreSeam,
    outbox: FakeOutboxPublisher,
}

impl ReadTestApi {
    /// Neue, leere TestApi.
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            calendar_seam: FakeCalendarSeam,
            money_seam: FakeMoneySeam,
            store_seam: FakeStoreSeam::default(),
            outbox: FakeOutboxPublisher::default(),
        }
    }

    /// Arrangiert eine Rohzeile (DTO) in der Quelle (in Lesereihenfolge).
    pub fn with_line(mut self, record: RawRecord) -> Self {
        self.lines.push(record);
        self
    }

    /// Fährt die echte Bridge-Kette für eine `ReadTransactions`-Nachricht der gegebenen Quelle
    /// und gibt das Outbox-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` = Dead-Letter).
    pub fn run(&self, source: &str) -> Result<(), OutboxError> {
        let content = serde_json::to_string(&ReadTransactionsMessage {
            source: source.to_string(),
            lines: self.lines.clone(),
        })
        .map_err(|error| OutboxError::Critical {
            message: format!("ReadTransactions not serializable: {error}"),
            source: read_transactions_message::MESSAGE_TYPE.to_string(),
        })?;
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
        let handler = ReadHandler::new(&calendar, &money, &store, &publisher);
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

    /// Die Partition-Keys der veröffentlichten Outbox-Kommandos in Reihenfolge.
    pub fn published_partition_keys(&self) -> Vec<String> {
        self.outbox.partition_keys()
    }

    /// Die `ordering_account`-Werte der **gestageten** Aggregate am Speicher-Seam, in
    /// Reihenfolge — so prüft ein Spec, dass je gültiger Zeile persistiert wurde.
    pub fn stored_ordering_accounts(&self) -> Vec<String> {
        self.store_seam.ordering_accounts()
    }
}

impl Default for ReadTestApi {
    fn default() -> Self {
        Self::new()
    }
}
