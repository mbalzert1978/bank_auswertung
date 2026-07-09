use obfuskierung_domain::{
    AccountToken, NonEmptyString, ObfuscatedText as DomainText, ObfuscationHandler, Obfuscator,
    Pseudonym as DomainPseudonym, Replacement, Seed,
};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

use super::fakes::{FakeIngestedSource, FakeObfuscatedStore, FakeObfuscator, FakeOutboxPublisher};
use crate::bridges::{
    CONSUMED_MESSAGE_TYPE, ConsumeBridge, ObfuscatorBridge, PublisherBridge, RepositoryBridge,
    SourceBridge,
};
use crate::dto::{AccountId, IngestedRow, ObfuscatedBooking, ObfuscatedText, Pseudonym};

/// Fluent-TestApi des Obfuskierungs-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// die **Eingangsfläche spricht nur DTOs** — mit [`with_ingested`](Self::with_ingested) arrangiert
/// man eine rohe einlesen-Zeile im (Fake-)einlesen-Speicher unter einer ID, [`run`](Self::run) fährt
/// dann für den **ID-Zeiger** die **echte** Kette Consume-Bridge → Handler (liest per ID →
/// obfuskiert → legt ab → publiziert ID) gegen öffentliche Fakes (Vorbild `ReadTestApi`).
///
/// **Erwartungen prüft die TestApi selbst** (`assert_*`): die Spec formuliert das erwartete
/// Ergebnis als **DTO** — die Token-Felder über die deterministischen `expected_*`-Helfer (dieselbe
/// Fake-Vergabe wie der Lauf) —, und die TestApi validiert die **ganze** abgelegte Buchung sowie den
/// publizierten Zeiger dagegen. Rohe Beobachter liegen für Eigenschafts-/Negativ-Prüfungen daneben.
pub struct ObfuscationTestApi {
    seed: u64,
    obfuscator: FakeObfuscator,
    source: FakeIngestedSource,
    store: FakeObfuscatedStore,
    outbox: FakeOutboxPublisher,
}

impl ObfuscationTestApi {
    /// Neue TestApi mit Seed `0` — reale Specs setzen ihn über [`with_seed`](Self::with_seed).
    pub fn new() -> Self {
        Self {
            seed: 0,
            obfuscator: FakeObfuscator,
            source: FakeIngestedSource::default(),
            store: FakeObfuscatedStore::default(),
            outbox: FakeOutboxPublisher::default(),
        }
    }

    /// Injiziert den Seed (den io später aus der Config liest) — er keyt die deterministische
    /// Vergabe und wird in jede abgelegte Buchung mitpersistiert.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Arrangiert eine rohe einlesen-Zeile im (Fake-)einlesen-Speicher unter ihrer ID — die der
    /// Handler beim Lauf per ID liest.
    pub fn with_ingested(mut self, id: &str, row: IngestedRow) -> Self {
        self.source.insert(id, row);
        self
    }

    /// Fährt die echte Bridge-Kette für einen `Ingested`-**Zeiger** (die ID reist in der Hülle, kein
    /// Inhalt) und gibt das Outbox-Ergebnis zurück (`Ok(())` = verarbeitet, `Err` = Retry/Dead-Letter).
    pub fn run(&self, id: &str) -> Result<(), OutboxError> {
        let obfuscator = ObfuscatorBridge {
            seam: &self.obfuscator,
        };
        let source = SourceBridge { seam: &self.source };
        let repository = RepositoryBridge { seam: &self.store };
        let publisher = PublisherBridge {
            outbox: &self.outbox,
        };
        let handler = ObfuscationHandler::new(
            Seed::new(self.seed),
            &source,
            &obfuscator,
            &repository,
            &publisher,
        );
        let bridge = ConsumeBridge { handler };
        let message = OutboxMessage {
            id: id.to_string(),
            message_type: CONSUMED_MESSAGE_TYPE.to_string(),
            partition_key: id.to_string(),
            content: String::new(),
            occurred_on_utc: 0,
            attempt: 0,
        };
        bridge.handle(message)
    }

    // --- Erwartete Token (deterministisch über dieselbe Fake-Vergabe wie der Lauf) ----------

    /// Das erwartete Konto-Token für eine IBAN unter dem gesetzten Seed — so formuliert eine Spec die
    /// Token-Felder ihres erwarteten DTOs, ohne den Fake-Hash zu kennen.
    pub fn expected_account(&self, iban: &str) -> AccountId {
        AccountId::from_token(&AccountToken::from_replacement(self.replacement(iban)))
    }

    /// Das erwartete Personen-Pseudonym für einen Namen unter dem gesetzten Seed.
    pub fn expected_pseudonym(&self, name: &str) -> Pseudonym {
        Pseudonym::from_domain(&DomainPseudonym::from_replacement(self.replacement(name)))
    }

    /// Der erwartete obfuskierte Freitext (getrimmt; Scrubbing vertagt) — nicht seed-abhängig.
    pub fn expected_text(&self, text: &str) -> ObfuscatedText {
        let value = NonEmptyString::new(text).expect("nicht-leerer Freitext");
        ObfuscatedText::from_domain(&DomainText::from(value))
    }

    fn replacement(&self, plaintext: &str) -> Replacement {
        let value = NonEmptyString::new(plaintext).expect("nicht-leerer Klartext");
        let obfuscator = ObfuscatorBridge {
            seam: &self.obfuscator,
        };
        obfuscator.obfuscate(Seed::new(self.seed), &value)
    }

    // --- Validierung des Ergebnisses ---------------------------------------------------------

    /// Prüft, dass **genau die** erwartete obfuskierte Buchung in der `obfuskiert`-Tabelle liegt
    /// (ganze DTO, ein Vergleich).
    pub fn assert_stored(&self, expected: ObfuscatedBooking) {
        assert_eq!(self.store.bookings(), vec![expected]);
    }

    /// Prüft, dass genau die erwarteten IDs unter dem Dispatch-Key `Obfuscated` als Zeiger publiziert
    /// wurden (Reihenfolge signifikant).
    pub fn assert_published(&self, ids: &[&str]) {
        let expected_types = vec!["Obfuscated".to_string(); ids.len()];
        let expected_ids: Vec<String> = ids.iter().map(|id| (*id).to_string()).collect();
        assert_eq!(self.outbox.message_types(), expected_types);
        assert_eq!(self.outbox.partition_keys(), expected_ids);
    }

    /// Prüft, dass **nichts** abgelegt oder publiziert wurde (Dead-Letter-/Retry-Fall).
    pub fn assert_nothing_emitted(&self) {
        assert!(
            self.store.bookings().is_empty(),
            "es wurde eine Buchung abgelegt"
        );
        assert!(
            self.outbox.partition_keys().is_empty(),
            "es wurde ein Zeiger publiziert"
        );
    }

    // --- Rohe Beobachter (für Eigenschafts-/Negativ-Prüfungen) -------------------------------

    /// Die in der `obfuskiert`-Tabelle abgelegten Buchungen, in Reihenfolge.
    pub fn stored_bookings(&self) -> Vec<ObfuscatedBooking> {
        self.store.bookings()
    }
}

impl Default for ObfuscationTestApi {
    fn default() -> Self {
        Self::new()
    }
}
