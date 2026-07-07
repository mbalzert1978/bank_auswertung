use std::fmt;

use shared_kernel::OutboxError;

use crate::{Rohsatz, Zeile};

/// Die io-facing Rohquelle: liefert die Rohzeilen **einer** Quelle (im echten io eine
/// CSV-CAMT-v8-Datei; die Tests unterschieben einen Fake). Ein Lesefehler ist **transient**
/// — Platte/Netz können kurzzeitig weg sein — und wird über [`RohquelleError`] zu
/// `OutboxError::Retry`. Der Adapter fordert hier nur an; **wie** eine Quelle adressiert und
/// gelesen wird, ist Sache des io-Crate, das diesen Port erfüllt.
pub trait Rohquelle {
    /// Liest alle Rohzeilen der adressierten Quelle, oder scheitert transient.
    fn lies(&self, quelle: &str) -> Result<Vec<Rohsatz>, RohquelleError>;
}

/// Der io-facing Zeilenspeicher: stagt eine validierte [`Zeile`] als Aggregat-Zustand. Im
/// Outbox-Pattern wird sie **gemeinsam** mit ihrem `Eingelesen`-Command in einer Transaktion
/// persistiert (der Commit gehört io); dieser Port trägt nur das Stagen.
pub trait Zeilenspeicher {
    /// Stagt die Zeile zur atomaren Persistenz (Commit besorgt der io-Prozessor).
    fn stage(&self, zeile: &Zeile);
}

/// Der **transiente** Lesefehler der [`Rohquelle`]: Platte/Netz kurzzeitig weg. Er ist die
/// retryable Fehlerquelle des Slice und wird über `From` auf `OutboxError::Retry` gemappt
/// (nächster Zyklus). Davon getrennt ist der **terminale** Aggregat-Fehler einer gelesenen
/// Zeile (`einlesen_domain::ZeilenError`): der bubblet als Dead-Letter auf (→ `Warning`), er
/// wird **nicht** hier getragen. Beide vereint der `EinleseError` des Handlers.
#[derive(Debug)]
pub struct RohquelleError {
    grund: String,
}

impl RohquelleError {
    /// Baut den transienten Lesefehler aus seinem menschenlesbaren Grund (io konstruiert ihn).
    pub fn new(grund: impl Into<String>) -> Self {
        Self {
            grund: grund.into(),
        }
    }
}

impl fmt::Display for RohquelleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rohquelle transient nicht lesbar: {}", self.grund)
    }
}

impl std::error::Error for RohquelleError {}

impl From<RohquelleError> for OutboxError {
    fn from(fehler: RohquelleError) -> Self {
        OutboxError::Retry {
            reason: fehler.grund,
        }
    }
}
