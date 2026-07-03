use shared_kernel::{OutboxError, OutboxMessage};

/// Typisierte Bau-Basis für eine **Consume-Bridge** (Vorbild
/// `Outbox/Adapter/OutboxHandlerBase.cs`, dort „Base" als OOP-Lehnwort — hier der
/// *typisierte* Handler): wer die zwei Bausteine `deserialize`/`dispatch` liefert,
/// bekommt die Standard-Verdrahtung
/// [`handle_outbox`](TypedOutboxHandler::handle_outbox) als Default-Methode
/// (Envelope → Request → `Result`, leerer Inhalt →
/// [`OutboxError::Critical`](shared_kernel::OutboxError::Critical)) — die
/// Dead-Letter-Meldung lautet so über alle Slices gleich. Die **Deserialisierung
/// bleibt abstrakt** (der Kernel ist serde-frei; die Bridge im Adapter füllt sie
/// mit der [`outbox_payload`](shared_kernel::outbox_payload)-Konvention).
///
/// **Kein Blanket-Impl `impl<T: TypedOutboxHandler> OutboxHandler for T`** (anders
/// als solange beide Traits in einer Crate lagen): seit
/// [`OutboxHandler`](shared_kernel::OutboxHandler) io-facing im `shared_kernel` und
/// dieser Trait slice-intern hier liegt, verböte die Orphan-Regel (E0210) den
/// Blanket-Impl über einen generischen `T`. Der Slice-Adapter implementiert
/// `OutboxHandler` daher für seinen **konkreten** Bridge-Typ (erlaubt — lokaler
/// Typ) und delegiert dessen `handle` an
/// [`handle_outbox`](TypedOutboxHandler::handle_outbox) — ein Dreizeiler, während
/// die eigentliche Verdrahtung genau **einmal** hier lebt.
///
/// **Kein `map_outcome` mehr** (anders als die C#-Basis): `dispatch` gibt selbst ein
/// `Result` zurück, und der Domänen-Fehler wird über `Into<OutboxError>` gemappt —
/// das erledigt in Rust, wofür drüben eine eigene Mapping-Methode nötig war.
pub trait TypedOutboxHandler {
    /// Der aus dem Envelope deserialisierte interne Request (Domänen-Kommando).
    type Request;
    /// Der Domänen-Fehler des Dispatch; wird über `Into` auf [`OutboxError`]
    /// gemappt. Die konkrete Zuordnung (Retry vs. Dead-Letter) trifft die Bridge
    /// im Adapter mit ihrem `From`-Impl — nicht der Kernel.
    type Error: Into<OutboxError>;

    /// Deserialisiert [`OutboxMessage::content`] zum internen Request; `None` bei
    /// leerem/ungültigem Inhalt (wird zu
    /// [`OutboxError::Critical`](shared_kernel::OutboxError::Critical) gemappt).
    fn deserialize(&self, content: &str) -> Option<Self::Request>;

    /// Dispatcht den Request an den internen Domänen-Handler; `Ok(())` = verarbeitet,
    /// `Err` = Domänen-Fehler (wird über `Into<OutboxError>` zur Disposition).
    fn dispatch(&self, request: Self::Request) -> Result<(), Self::Error>;

    /// Standard-Verdrahtung Envelope → Request → `Result`: deserialisiert den Inhalt
    /// (leer/ungültig → [`OutboxError::Critical`](shared_kernel::OutboxError::Critical)),
    /// dispatcht und mappt den Domänen-Fehler über `Into`. Der Slice-Adapter ruft
    /// dies aus seinem `impl OutboxHandler for <Bridge>` heraus — die Policy lebt so
    /// einmal hier, statt in jedem Slice wiederholt zu werden.
    fn handle_outbox(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        let request = self
            .deserialize(&message.content)
            .ok_or_else(|| OutboxError::Critical {
                message: format!(
                    "Outbox-Nachricht {} trägt leeren oder ungültigen Inhalt.",
                    message.id
                ),
                source: message.message_type,
            })?;
        self.dispatch(request).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ein minimaler Consume-Bridge-Fake: `deserialize` nimmt jeden nicht-leeren
    /// Inhalt an (leer → `None`), `dispatch` meldet für den Inhalt `"ok"` Erfolg und
    /// für jeden anderen einen Domänen-Fehler — so wird allein die Verdrahtung der
    /// Default-Methode [`handle_outbox`](TypedOutboxHandler::handle_outbox) geprüft
    /// (Ok-Durchreichen, Fehler-Mapping, Leer-Policy), nicht echte Serde-Logik.
    struct FakeBridge;

    /// Fake-Domänenfehler, der wie ein echter Slice-Fehler über `Into<OutboxError>`
    /// auf eine Disposition (hier Retry) gemappt wird.
    struct FakeFehler(String);

    impl From<FakeFehler> for OutboxError {
        fn from(fehler: FakeFehler) -> Self {
            OutboxError::Retry { reason: fehler.0 }
        }
    }

    impl TypedOutboxHandler for FakeBridge {
        type Request = String;
        type Error = FakeFehler;

        fn deserialize(&self, content: &str) -> Option<Self::Request> {
            (!content.is_empty()).then(|| content.to_string())
        }

        fn dispatch(&self, request: Self::Request) -> Result<(), Self::Error> {
            if request == "ok" {
                Ok(())
            } else {
                Err(FakeFehler(request))
            }
        }
    }

    fn nachricht(message_type: &str, content: &str) -> OutboxMessage {
        OutboxMessage {
            id: "id-1".to_string(),
            message_type: message_type.to_string(),
            partition_key: "konto-1".to_string(),
            content: content.to_string(),
            occurred_on_utc: 0,
            attempt: 0,
        }
    }

    #[test]
    fn verarbeiteter_inhalt_wird_zu_ok() {
        let outcome = FakeBridge.handle_outbox(nachricht("Eingelesen", "ok"));
        assert_eq!(
            outcome,
            Ok(()),
            "erfolgreicher dispatch muss als Ok(()) durchgereicht werden"
        );
    }

    #[test]
    fn dispatch_fehler_wird_ueber_into_auf_outbox_error_gemappt() {
        let outcome = FakeBridge.handle_outbox(nachricht("Eingelesen", "kaputt"));
        assert_eq!(
            outcome,
            Err(OutboxError::Retry {
                reason: "kaputt".to_string()
            }),
            "der Domänen-Fehler muss über Into<OutboxError> zur Disposition gemappt werden"
        );
    }

    #[test]
    fn leerer_inhalt_wird_zu_critical_mit_der_nachrichtenquelle() {
        let outcome = FakeBridge.handle_outbox(nachricht("Eingelesen", ""));
        assert!(
            matches!(
                &outcome,
                Err(OutboxError::Critical { source, .. }) if source == "Eingelesen"
            ),
            "leerer Inhalt muss zu Critical mit dem Dispatch-Key als Quelle führen, war: {outcome:?}"
        );
    }
}
