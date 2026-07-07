use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use super::fakes::{FakeKalenderSeam, FakeOutboxPublisher};
use crate::consume_bridge::ConsumeBridge;
use crate::eingelesen_publisher::EingelesenPublisher;
use crate::handler::EinleseHandler;
use crate::kalender_bridge::KalenderBridge;
use crate::rohsatz::Rohsatz;
use crate::umsaetze_einlesen::{self, UmsätzeEinlesen};

/// Fluent-TestApi des einlesen-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// Rohzeilen arrangieren, dann `ausführen` eine **echte** Kette Consume-Bridge →
/// Kalender-Bridge + [`EinleseHandler`] → Publisher-Bridge gegen öffentliche Fakes fahren
/// (Vorbild `DownloaderTestApi`: `RunAsync` verdrahtet die echten Bridges gegen Fakes der
/// public Abstractions, nie gegen interne Domänen-Ports direkt). Beobachtet wird über die
/// veröffentlichten `Eingelesen`-Commands (Fan-out, über ihren Dispatch-Key). Eine ungültige
/// Zeile lässt `ausführen` **terminal** mit `OutboxError::Warning` scheitern (die io-Outbox
/// dead-lettert dann die Nachricht); nichts wird veröffentlicht.
pub struct EinlesenTestApi {
    zeilen: Vec<Rohsatz>,
    kalender_seam: FakeKalenderSeam,
    outbox: FakeOutboxPublisher,
}

impl EinlesenTestApi {
    /// Neue, leere TestApi.
    pub fn new() -> Self {
        Self {
            zeilen: Vec::new(),
            kalender_seam: FakeKalenderSeam,
            outbox: FakeOutboxPublisher::default(),
        }
    }

    /// Arrangiert eine Rohzeile in der Quelle (in Lesereihenfolge).
    pub fn mit_zeile(mut self, rohsatz: Rohsatz) -> Self {
        self.zeilen.push(rohsatz);
        self
    }

    /// Fährt die echte Bridge-Kette für eine `UmsätzeEinlesen`-Nachricht der gegebenen Quelle
    /// und gibt das Outbox-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` = Dead-Letter).
    pub fn ausführen(&self, quelle: &str) -> Result<(), OutboxError> {
        let content = serde_json::to_string(&UmsätzeEinlesen {
            quelle: quelle.to_string(),
            zeilen: self.zeilen.clone(),
        })
        .map_err(|fehler| OutboxError::Critical {
            message: format!("UmsätzeEinlesen nicht serialisierbar: {fehler}"),
            source: umsaetze_einlesen::MESSAGE_TYPE.to_string(),
        })?;
        self.ausführen_mit_payload(quelle, content)
    }

    /// Fährt dieselbe Bridge-Kette wie [`ausführen`](Self::ausführen), aber mit
    /// frei vorgegebenem Outbox-`content` (z. B. für ungültige Payload-Tests).
    pub fn ausführen_mit_payload(&self, quelle: &str, content: String) -> Result<(), OutboxError> {
        let kalender = KalenderBridge {
            seam: &self.kalender_seam,
        };
        let publisher = EingelesenPublisher {
            outbox: &self.outbox,
        };
        let handler = EinleseHandler {
            kalender: &kalender,
            publisher: &publisher,
        };
        let bridge = ConsumeBridge { handler };
        let message = OutboxMessage {
            id: "test-nachricht".to_string(),
            message_type: umsaetze_einlesen::MESSAGE_TYPE.to_string(),
            partition_key: quelle.to_string(),
            content,
            occurred_on_utc: 0,
            attempt: 0,
        };
        bridge.handle(message)
    }

    /// Die veröffentlichten Outbox-Message-Types (z. B. `Eingelesen`) in
    /// Reihenfolge.
    pub fn veröffentlichte_message_types(&self) -> Vec<String> {
        self.outbox.message_types()
    }

    /// Die Partition-Keys der veröffentlichten Outbox-Kommandos in Reihenfolge.
    pub fn veröffentlichte_partition_keys(&self) -> Vec<String> {
        self.outbox.partition_keys()
    }
}

impl Default for EinlesenTestApi {
    fn default() -> Self {
        Self::new()
    }
}
