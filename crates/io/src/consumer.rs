use std::marker::PhantomData;

use application_kernel::{CommandDecoder, Handler};
use shared_kernel::{OutboxError, OutboxHandler, OutboxMessage, UnitOfWork};

/// Öffnet **eine frische [`UnitOfWork`] pro Nachricht** — die Transaktion, in der die
/// [`OutboxConsumer`] Consume-Ack, gestagete Stores und gestagete Publishes **atomar** committet.
/// Heute erfüllt sie der Integrationstest mit einem Fake; die echte io-Anbindung backt sie später
/// mit einer `sqlx`-Transaktion (Build-Order Schritt 5), ohne dass die Consumer-Maschinerie sich
/// ändert.
pub trait UnitOfWorkFactory {
    /// Die Arbeitseinheit, die [`begin`](UnitOfWorkFactory::begin) öffnet.
    type Unit: UnitOfWork;

    /// Öffnet die Arbeitseinheit für die anstehende Nachricht.
    fn begin(&self) -> Self::Unit;
}

/// Der **eine** generische Outbox-Consumer ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung A/B): er implementiert `shared_kernel::OutboxHandler` über **jeden**
/// `application_kernel::Handler` `H` und dessen slice-gelieferten [`CommandDecoder`] `D`. Statt
/// je Slice eine Consume-Bridge lebt die Verdrahtung Envelope → Kommando → `Result` **einmal**
/// hier.
///
/// **Ablauf je Nachricht:** die Envelope extrahieren und dem [`CommandDecoder`] nur die nackten
/// Strings `partition_key`/`content` reichen (Shape Y — der Slice nennt keinen Outbox-Typ) → eine
/// frische [`UnitOfWork`] öffnen → den Handler (staged) dispatchen → **einmal** committen. So
/// fallen aus dem Pattern at-least-once + wiederaufsetzbar; scheitert der Handler, wird **nicht**
/// committet — die Arbeitseinheit fällt zurück.
///
/// **Fehler-Taktik (io-intern, ADR-0011 Entscheidung B):** `OutboxError` wird **nur** hier
/// gebaut. Leerer/ungültiger Inhalt (Decoder `None`) → `Critical` (Envelope-/Dispatch-Ebene). Ein
/// Handler-Fehler ist **terminal** → `Warning` über das **`Display`** des Fehlers (nie `Debug`) —
/// den redigierten Meldungssatz besitzt allein der Slice-Fehler, `io` spiegelt ihn. Ein Commit-
/// Fehler → `Retry` (io-interner Broker-/Commit-Retry) — der **einzige** Retry-Ursprung, nie ein
/// Handler-Fehler.
pub struct OutboxConsumer<H, U, D> {
    handler: H,
    unit_of_work: U,
    decoder: PhantomData<fn() -> D>,
}

impl<H, U, D> OutboxConsumer<H, U, D>
where
    H: Handler,
    U: UnitOfWorkFactory,
    D: CommandDecoder<Command = H::Command>,
{
    /// Verdrahtet den Consumer über einem konkreten Handler und der Unit-of-Work-Quelle. Der
    /// Decoder `D` ist zustandslos (assoziierte Funktion, Shape Y) und lebt nur im Typ.
    pub fn new(handler: H, unit_of_work: U) -> Self {
        Self {
            handler,
            unit_of_work,
            decoder: PhantomData,
        }
    }
}

impl<H, U, D> OutboxHandler for OutboxConsumer<H, U, D>
where
    H: Handler,
    U: UnitOfWorkFactory,
    D: CommandDecoder<Command = H::Command>,
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

        let unit = self.unit_of_work.begin();
        self.handler
            .handle(command)
            .map_err(|error| OutboxError::Warning {
                reason: error.to_string(),
            })?;
        unit.commit().map_err(|error| OutboxError::Retry {
            reason: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::fmt;

    use application_kernel::Command;

    use super::*;

    /// Feldloses Test-Kommando (der Decoder baut es bzw. verwirft es).
    struct Cmd;
    impl Command for Cmd {}

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

    struct OkHandler;
    impl Handler for OkHandler {
        type Command = Cmd;
        type Response = ();
        type Error = DomainError;
        fn handle(&self, _command: Cmd) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct FailHandler;
    impl Handler for FailHandler {
        type Command = Cmd;
        type Response = ();
        type Error = DomainError;
        fn handle(&self, _command: Cmd) -> Result<(), DomainError> {
            Err(DomainError)
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

    struct OkUnitFactory;
    impl UnitOfWorkFactory for OkUnitFactory {
        type Unit = OkUnit;
        fn begin(&self) -> OkUnit {
            OkUnit
        }
    }
    struct OkUnit;
    impl UnitOfWork for OkUnit {
        type Error = Infallible;
        fn commit(self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    struct FailUnitFactory;
    impl UnitOfWorkFactory for FailUnitFactory {
        type Unit = FailUnit;
        fn begin(&self) -> FailUnit {
            FailUnit
        }
    }
    struct FailUnit;
    impl UnitOfWork for FailUnit {
        type Error = CommitError;
        fn commit(self) -> Result<(), CommitError> {
            Err(CommitError)
        }
    }
    #[derive(Debug)]
    struct CommitError;
    impl fmt::Display for CommitError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "commit gescheitert")
        }
    }
    impl std::error::Error for CommitError {}

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
        let consumer = OutboxConsumer::<_, _, NoneDecoder>::new(OkHandler, OkUnitFactory);
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
        let consumer = OutboxConsumer::<_, _, SomeDecoder>::new(FailHandler, OkUnitFactory);
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
        let consumer = OutboxConsumer::<_, _, SomeDecoder>::new(OkHandler, FailUnitFactory);
        assert!(
            matches!(
                consumer.handle(nachricht("Eingelesen")),
                Err(OutboxError::Retry { .. })
            ),
            "ein Commit-Fehler ist der einzige io-interne Retry-Ursprung"
        );
    }

    #[test]
    fn erfolg_dekodiert_dispatcht_und_committet() {
        let consumer = OutboxConsumer::<_, _, SomeDecoder>::new(OkHandler, OkUnitFactory);
        assert_eq!(consumer.handle(nachricht("Eingelesen")), Ok(()));
    }
}
