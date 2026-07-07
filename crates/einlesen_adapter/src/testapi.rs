use std::cell::RefCell;

use einlesen_domain::{Kalender, Kalendertag, NonEmptyString};
use shared_kernel::{OutboxCommand, OutboxError, OutboxHandler, OutboxMessage, OutboxPublisher};

use crate::Zeile;
use crate::consume_bridge::ConsumeBridge;
use crate::eingelesen_publisher::EingelesenPublisher;
use crate::handler::EinleseHandler;
use crate::ports::{Rohquelle, RohquelleError, Zeilenspeicher};
use crate::rohsatz::Rohsatz;
use crate::umsaetze_einlesen::{self, UmsätzeEinlesen};

/// Fluent-TestApi des einlesen-Slice ([ADR-0002](../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// Rohzeilen (und optional einen transienten Quellenfehler) arrangieren, dann `ausführen`
/// eine **echte** Kette Consume-Bridge → [`EinleseHandler`] → Port-/Publisher-Bridges gegen
/// öffentliche Fakes fahren. Beobachtet wird über die public DTOs: gestagte [`Zeile`]n
/// (Inhaltstreue) und veröffentlichte `Eingelesen`-Commands (Fan-out, über ihren Dispatch-Key)
/// — alles ohne EF/DI/Interna. Eine ungültige Zeile lässt `ausführen` **terminal** mit
/// `OutboxError::Warning` scheitern (die io-Outbox dead-lettert dann die Nachricht); nichts wird
/// gestaget oder veröffentlicht.
pub struct EinlesenTestApi {
    rohquelle: FakeRohquelle,
    kalender: FakeKalender,
    zeilenspeicher: FakeZeilenspeicher,
    outbox: FakeOutbox,
}

impl EinlesenTestApi {
    /// Neue, leere TestApi (mit deutschem Fake-Kalender).
    pub fn new() -> Self {
        Self {
            rohquelle: FakeRohquelle::default(),
            kalender: FakeKalender,
            zeilenspeicher: FakeZeilenspeicher::default(),
            outbox: FakeOutbox::default(),
        }
    }

    /// Arrangiert eine Rohzeile in der Quelle (in Lesereihenfolge).
    pub fn mit_zeile(mut self, rohsatz: Rohsatz) -> Self {
        self.rohquelle.zeilen.push(rohsatz);
        self
    }

    /// Lässt das Lesen der Quelle transient scheitern (→ `OutboxError::Retry`).
    pub fn mit_quellenfehler(mut self, grund: &str) -> Self {
        self.rohquelle.fehler = Some(grund.to_string());
        self
    }

    /// Fährt die echte Bridge-Kette für eine `UmsätzeEinlesen`-Nachricht der gegebenen Quelle
    /// und gibt das Outbox-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` = Retry/Dead-Letter).
    pub fn ausführen(&mut self, quelle: &str) -> Result<(), OutboxError> {
        let publisher = EingelesenPublisher {
            outbox: &self.outbox,
        };
        let handler = EinleseHandler {
            rohquelle: &self.rohquelle,
            kalender: &self.kalender,
            zeilenspeicher: &self.zeilenspeicher,
            publisher: &publisher,
        };
        let bridge = ConsumeBridge { handler };
        let content = serde_json::to_string(&UmsätzeEinlesen {
            quelle: quelle.to_string(),
        })
        .map_err(|fehler| OutboxError::Critical {
            message: format!("UmsätzeEinlesen nicht serialisierbar: {fehler}"),
            source: umsaetze_einlesen::MESSAGE_TYPE.to_string(),
        })?;
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

    /// Die gestagten Zeilen (Aggregat-Zustand) — Inhaltstreue-Beobachtung.
    pub fn gestagte_zeilen(&self) -> Vec<Zeile> {
        self.zeilenspeicher.gestagt.borrow().clone()
    }

    /// Die Dispatch-Keys der veröffentlichten `Eingelesen`-Commands — Fan-out-Beobachtung
    /// (`#Commands == #gestagte Zeilen`, die Invariante „Aggregat ⇔ Command atomar").
    pub fn veröffentlichte_eingelesen(&self) -> Vec<String> {
        self.outbox.veröffentlicht.borrow().clone()
    }
}

impl Default for EinlesenTestApi {
    fn default() -> Self {
        Self::new()
    }
}

// ── Fakes (crate-privat — nie über die Adapter-Grenze sichtbar) ───────────────────

/// Skriptbare Rohquelle: liefert die arrangierten Rohsätze, oder scheitert transient, wenn
/// ein Quellenfehler gesetzt ist.
#[derive(Default)]
struct FakeRohquelle {
    zeilen: Vec<Rohsatz>,
    fehler: Option<String>,
}

impl Rohquelle for FakeRohquelle {
    fn lies(&self, _quelle: &str) -> Result<Vec<Rohsatz>, RohquelleError> {
        match &self.fehler {
            Some(grund) => Err(RohquelleError::new(grund.clone())),
            None => Ok(self.zeilen.clone()),
        }
    }
}

/// Deterministischer Fake-Kalender: deutet „TT.MM.JJJJ" (grobe Prüfung im `Kalendertag`);
/// ungültiges Format ergibt `None`. Hinter dem echten Port steckt später `chrono` in io.
struct FakeKalender;

impl Kalender for FakeKalender {
    fn deute(&self, roh: &NonEmptyString) -> Option<Kalendertag> {
        let mut teile = roh.as_str().split('.');
        let tag: u8 = teile.next()?.parse().ok()?;
        let monat: u8 = teile.next()?.parse().ok()?;
        let jahr: u16 = teile.next()?.parse().ok()?;
        Kalendertag::aus_bestandteilen(jahr, monat, tag)
    }
}

/// Fängt die gestagten Zeilen (spielt die vertagte Persistenz).
#[derive(Default)]
struct FakeZeilenspeicher {
    gestagt: RefCell<Vec<Zeile>>,
}

impl Zeilenspeicher for FakeZeilenspeicher {
    fn stage(&self, zeile: &Zeile) {
        self.gestagt.borrow_mut().push(zeile.clone());
    }
}

/// Fängt die veröffentlichten Kommandos über ihren Dispatch-Key (`partition_key`). Der Payload
/// bleibt beim generischen `OutboxCommand` verborgen (Rust kann `T` nicht casten) — die
/// Inhaltstreue prüft die TestApi über die gestagten Zeilen, die Fan-out-Zählung hier.
#[derive(Default)]
struct FakeOutbox {
    veröffentlicht: RefCell<Vec<String>>,
}

impl OutboxPublisher for FakeOutbox {
    fn publish<T: OutboxCommand>(&self, command: T) {
        self.veröffentlicht
            .borrow_mut()
            .push(command.partition_key().to_string());
    }
}
