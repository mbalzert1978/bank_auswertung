use application_kernel::{CommandDecoder, Handler};
use obfuskierung_domain::{
    AccountToken, NonEmptyString, ObfuscationError, Obfuscator, Pseudonym as DomainPseudonym,
    Replacement, Seed,
};

use super::fakes::{
    FakeIngestedSource, FakeKnownPersons, FakeObfuscatedStore, FakeObfuscator, FakePublisher,
};
use crate::adapter::ObfuscatorBridge;
use crate::handler::ObfuscatingHandler;
use crate::models::ObfuscateCommandDecoder;
use crate::models::{AccountId, IngestedRow, ObfuscatedBooking, ObfuscatedText, Pseudonym};

/// Fluent-TestApi des Obfuskierungs-Slice ([ADR-0002](../../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md),
/// [ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)): die
/// **Eingangsfläche spricht nur DTOs** — mit [`with_ingested`](Self::with_ingested) arrangiert man
/// eine rohe einlesen-Zeile im (Fake-)einlesen-Speicher unter einer ID, [`run`](Self::run) fährt dann
/// für den **ID-Zeiger** die **echte** Kette über den `application_kernel`-Vertrag —
/// [`ObfuscateCommandDecoder`] (Envelope-Strings → Kommando, Shape Y) → [`ObfuscatingHandler`]
/// (Quell-/Obfuskator-/Repository-/Publisher-Bridge + domänen-interner Handler) gegen öffentliche
/// Fakes. **Keine Outbox-Mechanik mehr:** der Slice ist eine reine Pipeline; die
/// `OutboxConsumer`/`UnitOfWork`/Dead-Letter-Taktik prüft der io-Integrationstest, nicht dieser Spec.
///
/// `run` gibt den **nackten Domänen-[`ObfuscationError`]** zurück (kein `OutboxError`): eine misslungene
/// Beschaffung (Zeile oder Namensliste), eine absente Zeile oder ein fehlendes Pflichtfeld lässt ihn
/// **terminal** scheitern;
/// nichts wird abgelegt/publiziert.
///
/// **Erwartungen prüft die TestApi selbst** (`assert_*`): die Spec formuliert das erwartete Ergebnis
/// als **DTO** — die Token-Felder über die deterministischen `expected_*`-Helfer (dieselbe
/// Fake-Vergabe wie der Lauf) —, und die TestApi validiert die **ganze** abgelegte Buchung sowie den
/// publizierten Zeiger dagegen. Rohe Beobachter liegen für Eigenschafts-/Negativ-Prüfungen daneben.
pub struct ObfuscationTestApi {
    seed: u64,
    obfuscator: FakeObfuscator,
    source: FakeIngestedSource,
    persons: FakeKnownPersons,
    store: FakeObfuscatedStore,
    publisher: FakePublisher,
}

impl ObfuscationTestApi {
    /// Neue TestApi mit Seed `0` — reale Specs setzen ihn über [`with_seed`](Self::with_seed).
    #[must_use]
    pub fn new() -> Self {
        Self {
            seed: 0,
            obfuscator: FakeObfuscator,
            source: FakeIngestedSource::default(),
            persons: FakeKnownPersons::default(),
            store: FakeObfuscatedStore::default(),
            publisher: FakePublisher::default(),
        }
    }

    /// Injiziert den Seed (den io später aus der Config liest) — er keyt die deterministische Vergabe
    /// und wird in jede abgelegte Buchung mitpersistiert.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Arrangiert eine rohe einlesen-Zeile im (Fake-)einlesen-Speicher unter ihrer ID — die der
    /// Handler beim Lauf per ID liest.
    #[must_use]
    pub fn with_ingested(mut self, id: &str, row: IngestedRow) -> Self {
        self.source.insert(id, row);
        self
    }

    /// Lässt das **Lesen** terminal scheitern (io-Diagnose durchgereicht) — der unerreichbare
    /// Lese-Speicher, kategorisch verschieden von der schlicht absenten ID.
    #[must_use]
    pub fn with_failing_source(mut self, message: &str) -> Self {
        self.source.fail_with(message);
        self
    }

    /// Arrangiert einen **bekannten Personen-Namen** — so, wie io ihn aus dem einlesen-Bestand oder der
    /// lokalen Namensliste liefert. Der Freitext-Engpass ersetzt ihn im Verwendungszweck.
    #[must_use]
    pub fn with_known_person(mut self, name: &str) -> Self {
        self.persons.insert(name);
        self
    }

    /// Lässt das **Beschaffen der Namensliste** terminal scheitern (io-Diagnose durchgereicht).
    #[must_use]
    pub fn with_failing_persons(mut self, message: &str) -> Self {
        self.persons.fail_with(message);
        self
    }

    /// Lässt das **Ablegen** terminal scheitern (io-Diagnose durchgereicht) — das Gegenstück zu den
    /// gleichnamigen Arrangeuren der drei anderen Slices.
    #[must_use]
    pub fn with_failing_store(mut self, message: &str) -> Self {
        self.store.fail_with(message);
        self
    }

    /// Fährt die echte Kette (Decoder → Handler) für einen `Ingested`-**Zeiger** (die ID reist als
    /// `partition_key` der Envelope, kein Inhalt) und gibt das Handler-Ergebnis zurück (`Ok(())` =
    /// verarbeitet, `Err` = terminaler [`ObfuscationError`]).
    pub fn run(&self, id: &str) -> Result<(), ObfuscationError> {
        // Der Zeiger ist der `partition_key`; der `content` ist leer (reiner Zeiger). Ein blanker
        // Zeiger (→ Decoder `None`) ist eine io-Envelope-Sorge (`Critical`), nicht Handler-Eingabe —
        // die Specs reichen stets einen echten Zeiger herein.
        let command =
            ObfuscateCommandDecoder::decode(id, "").expect("nicht-leerer Aggregat-Zeiger");
        let handler = ObfuscatingHandler::new(
            self.seed,
            &self.source,
            &self.obfuscator,
            &self.persons,
            &self.store,
            &self.publisher,
        );
        handler.handle(command)
    }

    // --- Erwartete Token (deterministisch über dieselbe Fake-Vergabe wie der Lauf) ----------

    /// Das erwartete Konto-Token für eine IBAN unter dem gesetzten Seed — so formuliert eine Spec die
    /// Token-Felder ihres erwarteten DTOs, ohne den Fake-Hash zu kennen.
    pub fn expected_account(&self, iban: &str) -> AccountId {
        AccountId::from(&AccountToken::from_replacement(self.replacement(iban)))
    }

    /// Das erwartete Personen-Pseudonym für einen Namen unter dem gesetzten Seed.
    pub fn expected_pseudonym(&self, name: &str) -> Pseudonym {
        Pseudonym::from(&DomainPseudonym::from_replacement(self.replacement(name)))
    }

    /// Der erwartete obfuskierte Freitext, **wörtlich** so, wie er abgelegt sein soll — die Spec
    /// schreibt Platzhalter (`[IBAN]`, `[BIC]`, `[PERSON]`) selbst hinein, statt die Scrubbing-Regeln
    /// hier nachzurechnen. Nicht seed-abhängig.
    pub fn expected_text(&self, text: &str) -> ObfuscatedText {
        ObfuscatedText::expected(text)
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
        let expected_ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
        assert_eq!(self.publisher.message_types(), expected_types);
        assert_eq!(self.publisher.partition_keys(), expected_ids);
    }

    /// Prüft, dass **nichts** abgelegt oder publiziert wurde (terminaler Fehler).
    pub fn assert_nothing_emitted(&self) {
        assert!(
            self.store.bookings().is_empty(),
            "es wurde eine Buchung abgelegt"
        );
        assert!(
            self.publisher.partition_keys().is_empty(),
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
