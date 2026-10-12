use super::outbox::{OutboxError, OutboxHandler, OutboxMessage, OutboxPublisher, UnitOfWork};
use application_kernel::CommandDecoder;

use crate::PublishingPort;

/// Was **eine Nachricht** in **einer** Arbeitseinheit tut: eine
/// [`TransactionalHandler`]-Implementierung öffnet die Unit-of-Work, baut den Slice-Handler
/// **aus ihr** (Store-/Publish-Ports = die Unit selbst), dispatcht das Kommando und committet
/// **atomar**, wenn der Handler gelingt. Weil Handler und Unit **zusammen je Aufruf** entstehen und
/// den Aufruf nie verlassen, committet eine gestagete Store-/Publish-Schreibung **mit** der Unit
/// oder gar nicht — die Atomarität des Outbox-Patterns (ADR-0011, Entscheidung A; `io` besitzt die
/// Unit-of-Work). Der echte `rusqlite`-`io` backt sie mit **einer** `rusqlite`-Transaktion, der Test mit
/// einer In-Memory-Unit.
///
/// Der [`Decoder`](TransactionalHandler::Decoder) (Shape Y, ACL) gehört zum Slice-Handler: er baut
/// das Kommando aus den nackten Envelope-Strings, ohne dass der Slice einen Outbox-Typ nennt.
pub trait TransactionalHandler {
    /// Das interne Kommando, das dieser Handler verarbeitet.
    type Command;
    /// Der slice-gelieferte Envelope-Decoder (Shape Y) für dieses Kommando.
    type Decoder: CommandDecoder<Command = Self::Command>;
    /// Der **nackte Domänen-Fehler** des Slice (redigierte `Display`, die `io` als Dead-Letter-
    /// Grund spiegelt).
    type Error: std::error::Error;

    /// Verarbeitet das Kommando in einer atomaren Arbeitseinheit und markiert **dieselbe** Einheit
    /// die konsumierte Nachricht `ack` als verarbeitet. Die Ack-ID kommt **explizit** vom
    /// [`OutboxConsumer`] und wird an die Arbeitseinheit weitergereicht (`begin(ack)`) — kein ambient
    /// geteilter Zustand.
    fn handle(
        &self,
        ack: MessageId<'_>,
        command: Self::Command,
    ) -> Result<(), TransactionError<Self::Error>>;
}

/// Die **Unit-Quelle** eines Slice: `begin` öffnet je Nachricht **eine** Arbeitseinheit, die
/// **zugleich** die gestagete Ablage des Slice **und** die Outbox ([`OutboxPublisher`]) ist und
/// beides atomar committet ([`UnitOfWork`]). Weil dieselbe Unit staged **und** committet, committet
/// eine gestagete Schreibung mit der Nachricht oder gar nicht. Der echte `io` backt sie mit **einer**
/// `rusqlite`-Transaktion ([`SliceStore`](crate::sqlite::SliceStore) erfüllt sie generisch für jeden
/// [`StoreSpec`](crate::sqlite::StoreSpec)), der Test mit einer In-Memory-Unit.
///
/// **Ein** Trait für alle vier Slices ([ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md),
/// [Issue 0077](../../../../docs/issues/0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md)):
/// die vormals vier strukturgleichen `<slice>::UnitFactory` unterschieden sich einzig im Ablage-Seam
/// auf `type Unit`. Der Bound hier trägt darum nur, was **jede** Einheit kann (publizieren +
/// committen); **welchen** Ablage-Seam eine Reichweite zusätzlich braucht, verlangt der jeweilige
/// [`TransactionalHandler`] dort, wo er ihn benutzt (`where F::Unit: …StoreSeam`) — die
/// fenster-übergreifende `auswerten`-Reichweite etwa legt nichts ab und verlangt darum auch nichts.
pub trait UnitFactory {
    /// Die Arbeitseinheit: gestagete Ablage + Outbox in einem, atomar committbar.
    type Unit: UnitOfWork + OutboxPublisher;

    /// Öffnet die Arbeitseinheit für die Nachricht `ack`, die sie beim Commit als verarbeitet
    /// markiert. Die Ack-ID kommt **explizit** als Argument (nicht aus ambient Zustand), damit die
    /// Unit eine reine Funktion ihres Arguments ist.
    fn begin(&self, ack: MessageId<'_>) -> Self::Unit;
}

/// Die Identität der Outbox-Nachricht, die eine Arbeitseinheit als **verarbeitet** markiert (ackt).
/// Ein eigener Typ statt eines nackten `&str`, damit die Ack-ID nicht mit `partition_key`/`content`
/// (ebenfalls Strings) verwechselt wird und der Datenfluss **explizit** durch die Aufrufkette läuft:
/// der [`OutboxConsumer`] liest sie aus der [`OutboxMessage::id`](super::outbox::OutboxMessage::id),
/// reicht sie an den [`TransactionalHandler`] und dieser an die Unit-Quelle (`begin(ack)`) — statt
/// sie über geteilten mutablen Zustand zu schmuggeln.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageId<'a>(pub &'a str);

/// Die Disposition **eines** transaktionalen Versuchs — von `io` erzeugt, nie von einem Slice.
pub enum TransactionError<E> {
    /// Der Handler ist **terminal** gescheitert (kaputte Daten / gebrochenes Invariant) →
    /// Dead-Letter. Trägt den nackten Domänen-Fehler; `io` spiegelt dessen `Display`.
    Handler(E),
    /// Der **Commit** ist gescheitert → io-interner Retry (nie aus einem Handler-Fehler). Trägt
    /// den bereits stringifizierten Grund (der Commit-Fehlertyp lebt nicht über diese Grenze).
    ///
    /// **Retry ist begrenzt** ([Issue 0085](../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)):
    /// der Broker ([`SqliteOutbox`](crate::sqlite::SqliteOutbox)) stellt die offene Nachricht erneut zu,
    /// bis [`MAX_DELIVERY_ATTEMPTS`](crate::sqlite::MAX_DELIVERY_ATTEMPTS) Versuche verbraucht sind, und
    /// sondert sie dann mit diesem Grund aus.
    ///
    /// **Redaktions-Pflicht:** wie die io-Diagnose des Quell-Seams
    /// beschreibt er **Transaktions-Mechanik** (Broker/DB), **nie** einen Roh-Zeilenwert — die reale
    /// `UnitOfWork::Error`-`Display` muss das einhalten (siehe
    /// [`SqliteCommitError`](crate::sqlite::SqliteCommitError)). Der `obfuskierung`-Integrationstest
    /// fährt diesen Pfad bereits (sein Fake-Commit scheitert mit einer reinen Transaktions-Diagnose
    /// „broker weg" und wird als einziger Retry-Fall gespiegelt); einlesens Fake-Commit ist noch
    /// `Infallible`.
    Commit(String),
}

/// Der **eine** generische Outbox-Consumer ([ADR-0011](../../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung A/B): er implementiert [`OutboxHandler`](super::outbox::OutboxHandler) über **jeden**
/// [`TransactionalHandler`] `T`. Statt je Slice eine Consume-Bridge lebt die Verdrahtung
/// Envelope → Kommando → Disposition **einmal** hier.
///
/// **Ablauf je Nachricht:** die Envelope extrahieren und dem slice-eigenen
/// [`CommandDecoder`](TransactionalHandler::Decoder) nur die nackten Strings
/// `partition_key`/`content` reichen (Shape Y — der Slice nennt keinen Outbox-Typ) → das Kommando
/// **zusammen mit der Ack-ID** ([`MessageId`], aus [`OutboxMessage::id`](super::outbox::OutboxMessage::id))
/// an den [`TransactionalHandler`] geben, der die Arbeitseinheit öffnet, den Handler aus ihr baut,
/// dispatcht und **einmal** committet — der Commit markiert dieselbe Nachricht als verarbeitet.
///
/// **Fehler-Taktik (io-intern, ADR-0011 Entscheidung B):** der `OutboxError` der
/// **Nachrichtenverarbeitung** entsteht hier — leerer/ungültiger Inhalt (Decoder `None`) →
/// `Critical` (Envelope-/Dispatch-Ebene); ein Handler-Fehler ist **terminal** → `Warning` über das
/// **`Display`** des Fehlers (nie `Debug`), dessen redigierten Meldungssatz allein der Slice
/// besitzt; ein Commit-Fehler → `Retry`, der **einzige** io-interne Retry-Ursprung. (Die
/// Auflösungs-Ebene baut ihr eigenes `Critical` im [`Dispatcher`](crate::Dispatcher), wenn kein
/// Consumer zum `message_type` passt.) Was danach mit der Nachricht geschieht — sofort aussondern
/// oder bis zur Höchstzahl erneut zustellen —, entscheidet der Broker
/// ([`SqliteOutbox::deliver_open`](crate::sqlite::SqliteOutbox::deliver_open), Issue 0085), nicht
/// dieser Consumer und erst recht nicht der Slice.
pub struct OutboxConsumer<T> {
    transaction: T,
}

impl<T: TransactionalHandler> OutboxConsumer<T> {
    /// Verdrahtet den Consumer über einem konkreten [`TransactionalHandler`].
    pub fn new(transaction: T) -> Self {
        Self { transaction }
    }
}

impl<T: TransactionalHandler> OutboxHandler for OutboxConsumer<T> {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        let command =
            T::Decoder::decode(&message.partition_key, &message.content).ok_or_else(|| {
                OutboxError::Critical {
                    message: format!(
                        "Outbox-Nachricht {} trägt leeren oder ungültigen Inhalt.",
                        message.id
                    ),
                    source: message.message_type,
                }
            })?;

        // Die Ack-ID fließt explizit weiter — die Arbeitseinheit markiert **diese** Nachricht als
        // verarbeitet, atomar mit ihren Schreibungen.
        self.transaction
            .handle(MessageId(&message.id), command)
            .map_err(OutboxError::from)
    }
}

/// Übersetzt die Disposition eines transaktionalen Versuchs in den `OutboxError` der
/// Nachrichtenverarbeitung (io-intern, ADR-0011 Entscheidung B): ein Handler-Fehler ist
/// **terminal** → `Warning` über das **`Display`** des nackten Domänen-Fehlers (nie `Debug`),
/// dessen redigierten Meldungssatz allein der Slice besitzt; ein Commit-Fehler → `Retry`, der
/// **einzige** io-interne Retry-Ursprung.
impl<E: std::fmt::Display> From<TransactionError<E>> for OutboxError {
    fn from(error: TransactionError<E>) -> Self {
        match error {
            TransactionError::Handler(error) => OutboxError::Warning {
                reason: error.to_string(),
            },
            TransactionError::Commit(reason) => OutboxError::Retry { reason },
        }
    }
}

/// Der **Commit-Rand** eines transaktionalen Slice-Laufs, **einmal** benannt statt je Slice
/// dupliziert (ADR-0011, Entscheidung A): öffnet den [`PublishingPort`] über der Unit, lässt den
/// Aufrufer den Slice-Handler **aus der Unit** bauen und laufen (`run`), und committet die Unit
/// **genau einmal** bei Erfolg. Ein Handler-Fehler ist terminal → [`TransactionError::Handler`]; nur
/// der Commit erzeugt [`TransactionError::Commit`] (der io-interne Retry-Ursprung).
///
/// Weil `run` den Handler baut **und** fährt, verlässt der Handler (mit seinen Borrows auf Unit und
/// Publisher) den Aufruf nie — eine gestagete Store-/Publish-Schreibung committet mit der Unit oder
/// gar nicht. Jede Slice-Verdrahtung ([`einlesen`](crate::einlesen)/[`obfuskierung`](crate::obfuskierung))
/// reicht nur ihren einen `XHandler::new(…).handle(command)`-Ausdruck als `run` herein; die
/// Begin-/Commit-Ordnung und das Fehler-Mapping leben hier.
pub(crate) fn run_in_unit<U, E>(
    unit: U,
    run: impl FnOnce(&U, &PublishingPort<'_, U>) -> Result<(), E>,
) -> Result<(), TransactionError<E>>
where
    U: UnitOfWork + OutboxPublisher,
{
    // Der innere Block gibt die Borrows auf die Unit (Store + Publisher) wieder frei, bevor `commit`
    // sie verbraucht.
    let outcome = {
        let publishing = PublishingPort::new(&unit);
        run(&unit, &publishing)
    };
    outcome.map_err(TransactionError::Handler)?;
    unit.commit()
        .map_err(|error| TransactionError::Commit(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::fmt;
    use std::marker::PhantomData;

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

    /// Ein [`TransactionalHandler`], dessen Decoder-Typ und Disposition der Test vorgibt.
    struct FakeTransaction<D> {
        outcome: fn() -> Result<(), TransactionError<DomainError>>,
        decoder: PhantomData<fn() -> D>,
    }
    impl<D: CommandDecoder<Command = Cmd>> TransactionalHandler for FakeTransaction<D> {
        type Command = Cmd;
        type Decoder = D;
        type Error = DomainError;
        fn handle(
            &self,
            _ack: MessageId<'_>,
            _command: Cmd,
        ) -> Result<(), TransactionError<DomainError>> {
            (self.outcome)()
        }
    }

    fn consumer<D: CommandDecoder<Command = Cmd>>(
        outcome: fn() -> Result<(), TransactionError<DomainError>>,
    ) -> OutboxConsumer<FakeTransaction<D>> {
        OutboxConsumer::new(FakeTransaction {
            outcome,
            decoder: PhantomData,
        })
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
