use application_kernel::Command;
use serde::{Deserialize, Serialize};

/// Der Dispatch-Key, auf den der Outbox-Prozessor die Consume-Bridge dieses Slice auflöst.
pub(crate) const MESSAGE_TYPE: &str = "UmsätzeEinlesen";

/// Das eingehende Kommando des Slice — der Kopf der Pipeline bekommt es (später) von einem
/// Bootstrap in die Outbox gelegt. Es trägt nur die **Kennung der Quelle** (Aggregat-/
/// Partition-Identität), nicht deren Inhalt; die Rohzeilen holt der Handler über die
/// [`Rohquelle`](crate::Rohquelle). Adapter-lokal und **nicht** `pub`: die Domäne bindet
/// `application_kernel` nicht (blind), also lebt der `Command`-Marker hier.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UmsätzeEinlesen {
    pub(crate) quelle: String,
}

impl Command for UmsätzeEinlesen {}
