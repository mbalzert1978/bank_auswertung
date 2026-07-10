/// Das **domänen-interne Kommando** des Slice, das der
/// [`ReadHandler`](crate::ReadHandler) verarbeitet — seit
/// [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
/// ein reiner **Auslöser**, kein Transporter: es trägt keine Zeilen mehr, nur die
/// [`RunConfiguration`] der Beschaffung. Den Batch beschafft der Handler selbst über den
/// von der Domäne verlangten Port [`RawLineSource`](crate::RawLineSource). Der Adapter
/// baut das Kommando aus der (feldlosen) Wire-Nachricht; die Domäne selbst kennt keine
/// Wire-/serde-Form.
pub struct ReadTransactions {
    pub(crate) configuration: RunConfiguration,
}

impl ReadTransactions {
    /// Baut den Auslöser mit der Konfiguration des Laufs (Adapter-Abbildung).
    pub fn new(configuration: RunConfiguration) -> Self {
        Self { configuration }
    }
}

/// Die Konfiguration eines Einlese-Laufs — sie parametrisiert die **Beschaffung** (der
/// Handler reicht sie an [`RawLineSource::fetch`](crate::RawLineSource::fetch) durch).
/// **Vorerst ein leerer Marker** (bewusst non-`Option`: „nichts konfiguriert" liegt im Typ
/// selbst): was sie trägt (Quellpfad? Format-Wahl?), entscheidet erst der io-Schritt.
#[derive(Debug, Clone, Default)]
pub struct RunConfiguration;
