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

    /// Belegt die **Zeiger-Semantik** (Tabelle = Wahrheit, Message = Pointer), **ohne** dass ein
    /// Spec die intern vom Speicher-Fake vergebenen IDs zu Gesicht bekommt (keine Kopplung an die
    /// Fake-Nummerierung): `true`, wenn die veröffentlichten „Eingelesen"-Zeiger **paarweise
    /// verschieden** sind und **genau** (in Lesereihenfolge) den Schlüsseln entsprechen, unter
    /// denen die Roh-Zeilen am Speicher-Seam abgelegt wurden. So prüft ein Spec das Verhalten
    /// — jeder Zeiger deckt genau eine abgelegte Zeile —, nicht die konkreten IDs. Die Fan-out-
    /// **Zahl** liest der Spec weiterhin an [`published_message_types`](Self::published_message_types).
    pub fn published_pointers_match_distinct_stored_rows(&self) -> bool {
        let zeiger = self.outbox.partition_keys();
        let verschieden: std::collections::HashSet<_> = zeiger.iter().collect();
        verschieden.len() == zeiger.len() && self.store_seam.stored_ids() == zeiger
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
