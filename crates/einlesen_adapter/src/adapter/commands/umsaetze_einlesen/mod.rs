use application_kernel::Command;
use serde::{Deserialize, Serialize};

use crate::rohsatz::Rohsatz;

/// Der Dispatch-Key, auf den der Outbox-Prozessor die Consume-Bridge dieses Slice auflöst.
pub(crate) const MESSAGE_TYPE: &str = "UmsätzeEinlesen";

/// Das eingehende Kommando des Slice: trägt **den ganzen Batch** — alle Rohzeilen der Quelle
/// ([ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md): „der Command
/// trägt, was der Handler für seinen Task braucht — für den Ein-Rutsch-Betrieb der ganze Batch,
/// nicht zeilenweise"). Adapter-lokal und **nicht** `pub`: die Domäne bindet
/// `application_kernel` nicht (blind), also lebt der `Command`-Marker hier.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UmsätzeEinlesen {
    pub(crate) quelle: String,
    pub(crate) zeilen: Vec<Rohsatz>,
}

impl Command for UmsätzeEinlesen {}
