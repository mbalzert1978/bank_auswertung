use std::marker::PhantomData;

use application_kernel::CommandDecoder;
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage};

/// Was **eine Nachricht** in **einer** Arbeitseinheit tut: eine
/// [`TransactionalHandler`]-Implementierung öffnet die Unit-of-Work, baut den Slice-Handler
/// **aus ihr** (sodass eine gestagete Store-/Publish-Schreibung nur in genau die Unit landen
/// kann, die committet), dispatcht das Kommando und committet **atomar**, wenn der Handler
/// gelingt. Weil Handler und Unit **zusammen je Aufruf** entstehen und den Aufruf nie verlassen,
/// ist der Zustand „gestaged außerhalb der committeten Transaktion" **unkonstruierbar** — genau
/// die Härtung, die das Outbox-Pattern braucht (ADR-0011, Entscheidung A; die Verortung der
/// Unit-of-Work in `io` bleibt, nur die Bindung an die Ports wird typ-erzwungen). Der echte
/// `sqlx`-`io` backt sie mit **einer** `sqlx`-Transaktion, der Test mit einer In-Memory-Unit.
pub trait TransactionalHandler {
    /// Das interne Kommando, das dieser Handler verarbeitet.
    type Command;
    /// Der **nackte Domänen-Fehler** des Slice (redigierte `Display`, die `io` als Dead-Letter-
    /// Grund spiegelt).
    type Error: std::error::Error;

    /// Verarbeitet das Kommando in einer atomaren Arbeitseinheit.
    fn handle(&self, command: Self::Command) -> Result<(), TransactionError<Self::Error>>;
}

/// Die Disposition **eines** transaktionalen Versuchs — von `io` erzeugt, nie von einem Slice.
pub enum TransactionError<E> {
    /// Der Handler ist **terminal** gescheitert (kaputte Daten / gebrochenes Invariant) →
    /// Dead-Letter. Trägt den nackten Domänen-Fehler; `io` spiegelt dessen `Display`.
    Handler(E),
    /// Der **Commit** ist gescheitert → io-interner Retry (nie aus einem Handler-Fehler). Trägt
    /// den bereits stringifizierten Grund (der Commit-Fehlertyp lebt nicht über diese Grenze).
    Commit(String),
}

/// Der **eine** generische Outbox-Consumer ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung A/B): er implementiert `shared_kernel::OutboxHandler` über **jeden**
/// [`TransactionalHandler`] `T` und dessen slice-gelieferten [`CommandDecoder`] `D`. Statt je
/// Slice eine Consume-Bridge lebt die Verdrahtung Envelope → Kommando → Disposition **einmal**
/// hier.
///
/// **Ablauf je Nachricht:** die Envelope extrahieren und dem [`CommandDecoder`] nur die nackten
/// Strings `partition_key`/`content` reichen (Shape Y — der Slice nennt keinen Outbox-Typ) → das
/// Kommando an den [`TransactionalHandler`] geben, der die Arbeitseinheit öffnet, den Handler aus
/// ihr baut, dispatcht und **einmal** committet.
///
/// **Fehler-Taktik (io-intern, ADR-0011 Entscheidung B):** `OutboxError` wird **nur** hier
/// gebaut. Leerer/ungültiger Inhalt (Decoder `None`) → `Critical` (Envelope-/Dispatch-Ebene). Ein
/// Handler-Fehler ist **terminal** → `Warning` über das **`Display`** des Fehlers (nie `Debug`) —
/// den redigierten Meldungssatz besitzt allein der Slice-Fehler, `io` spiegelt ihn. Ein Commit-
/// Fehler → `Retry` (io-interner Broker-/Commit-Retry) — der **einzige** Retry-Ursprung, nie ein
/// Handler-Fehler.
pub struct OutboxConsumer<T, D> {
    transaction: T,
    decoder: PhantomData<fn() -> D>,
}

impl<T, D> OutboxConsumer<T, D>
where
    T: TransactionalHandler,
    D: CommandDecoder<Command = T::Command>,
{
    /// Verdrahtet den Consumer über einem konkreten [`TransactionalHandler`]. Der Decoder `D` ist
    /// zustandslos (assoziierte Funktion, Shape Y) und lebt nur im Typ.
    pub fn new(transaction: T) -> Self {
        Self {
            transaction,
            decoder: PhantomData,
        }
    }
}

impl<T, D> OutboxHandler for OutboxConsumer<T, D>
where
    T: TransactionalHandler,
    D: CommandDecoder<Command = T::Command>,
{
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        let command = D::decode(&message.partition_key, &message.content).ok_or_else(|| {
            OutboxError::Critical {
                message: format!(
                    "Outbox-Nachricht {} trägt leeren oder ungültigen Inhalt.",
                    message.id
                ),
                source: message.message_type,
            }
        })?;

        // Terminal → Warning (Display, redigiert); Commit-Fehler → Retry (einziger io-interner
        // Retry-Ursprung). Die attempt-basierte Eskalation Retry → Dead-Letter nach erschöpften
        // Versuchen gehört dem Broker (`OutboxMessage::attempt`, Build-Order Schritt 5), nicht dem
        // Consumer.
        self.transaction
            .handle(command)
            .map_err(|error| match error {
                TransactionError::Handler(error) => OutboxError::Warning {
                    reason: error.to_string(),
                },
                TransactionError::Commit(reason) => OutboxError::Retry { reason },
            })
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::*;

    /// Feldloses Test-Kommando (der Decoder baut es bzw. verwirft es).
    struct Cmd;

    /// Ein redigierter Fake-Domänen-Fehler: seine `Display` ist der einzige Meldungssatz, den
    /// die `OutboxConsumer` als Dead-Letter-Grund spiegelt (nie `Debug`).
    #[derive(Debug)]
    struct DomainError;
    impl fmt::Display for DomainError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "redigierter grund")
        }
    }
    impl std::error::Error for DomainError {}

    /// Ein [`TransactionalHandler`], dessen Disposition der Test vorgibt.
    struct FakeTransaction {
        outcome: fn() -> Result<(), TransactionError<DomainError>>,
    }
    impl TransactionalHandler for FakeTransaction {
        type Command = Cmd;
        type Error = DomainError;
        fn handle(&self, _command: Cmd) -> Result<(), TransactionError<DomainError>> {
            (self.outcome)()
        }
    }

    /// Liefert stets ein Kommando (nicht-leerer/gültiger Inhalt).
    struct SomeDecoder;
    impl CommandDecoder for SomeDecoder {
        type Command = Cmd;
        fn decode(_partition_key: &str, _content: &str) -> Option<Cmd> {
            Some(Cmd)
        }
    }

    /// Liefert nie ein Kommando (leerer/ungültiger Inhalt).
    struct NoneDecoder;
    impl CommandDecoder for NoneDecoder {
        type Command = Cmd;
        fn decode(_partition_key: &str, _content: &str) -> Option<Cmd> {
            None
        }
    }

    fn consumer<D: CommandDecoder<Command = Cmd>>(
        outcome: fn() -> Result<(), TransactionError<DomainError>>,
    ) -> OutboxConsumer<FakeTransaction, D> {
        OutboxConsumer::new(FakeTransaction { outcome })
    }

    fn nachricht(message_type: &str) -> OutboxMessage {
        OutboxMessage {
            id: "id-1".to_string(),
            message_type: message_type.to_string(),
            partition_key: "konto-1".to_string(),
            content: String::new(),
            occurred_on_utc: 0,
            attempt: 0,
        }
    }

    #[test]
    fn leerer_oder_ungueltiger_inhalt_wird_zu_critical_mit_der_quelle() {
        let consumer = consumer::<NoneDecoder>(|| Ok(()));
        assert!(
            matches!(
                consumer.handle(nachricht("Eingelesen")),
                Err(OutboxError::Critical { source, .. }) if source == "Eingelesen"
            ),
            "leerer Inhalt muss zu Critical mit dem Dispatch-Key als Quelle führen"
        );
    }

    #[test]
    fn ein_handler_fehler_wird_terminal_zu_warning_ueber_display() {
        let consumer = consumer::<SomeDecoder>(|| Err(TransactionError::Handler(DomainError)));
        assert_eq!(
            consumer.handle(nachricht("Eingelesen")),
            Err(OutboxError::Warning {
                reason: "redigierter grund".to_string()
            }),
            "der nackte Domänen-Fehler muss terminal über sein Display zum Warning werden"
        );
    }

    #[test]
    fn ein_commit_fehler_wird_io_intern_zu_retry() {
        let consumer =
            consumer::<SomeDecoder>(|| Err(TransactionError::Commit("commit weg".to_string())));
        assert_eq!(
            consumer.handle(nachricht("Eingelesen")),
            Err(OutboxError::Retry {
                reason: "commit weg".to_string()
            }),
            "ein Commit-Fehler ist der einzige io-interne Retry-Ursprung"
        );
    }

    #[test]
    fn erfolg_dekodiert_dispatcht_und_committet() {
        let consumer = consumer::<SomeDecoder>(|| Ok(()));
        assert_eq!(consumer.handle(nachricht("Eingelesen")), Ok(()));
    }
}
