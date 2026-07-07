use application_kernel::{Command, PublishCommand};

use crate::Zeile;

/// Der Dispatch-Key des ausgehenden Commands — die **eine** Quelle, die
/// `PublishCommand::MESSAGE_TYPE` (hier) und `OutboxCommand::MESSAGE_TYPE` (auf der
/// [`EingelesenMessage`](crate::eingelesen_message::EingelesenMessage)) teilen.
pub(crate) const MESSAGE_TYPE: &str = "Eingelesen";

/// Das ausgehende Kommando **pro eingelesener Zeile** (Fan-out, kein Batch): trägt die
/// validierte [`Zeile`] weiter an den nachgelagerten `obfuskieren`-Slice. Adapter-lokal und
/// **nicht** `pub` — ein `impl PublishCommand` für einen Domänentyp wäre ein Orphan-Verstoß,
/// und die Domäne bindet `application_kernel` nicht. Die Publisher-Bridge setzt es auf die
/// serialisierbare `EingelesenMessage` um.
pub(crate) struct Eingelesen {
    pub(crate) zeile: Zeile,
}

impl Eingelesen {
    pub(crate) fn new(zeile: Zeile) -> Self {
        Self { zeile }
    }
}

impl Command for Eingelesen {}

impl PublishCommand for Eingelesen {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;
}
