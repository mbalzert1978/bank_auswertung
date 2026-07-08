use serde::Serialize;
use shared_kernel::OutboxCommand;

use crate::abstractions::CalendarDay;
use crate::dto::{Direction, Line};

/// Der Dispatch-Key des ausgehenden Commands — die **eine** Quelle, auf die der Outbox-Prozessor
/// des nachgelagerten Slice seine Consume-Bridge auflöst. Das Domänen-Aggregat `LineAggregate`
/// (in `einlesen_domain`) trägt keinen Dispatch-Key; er lebt hier auf dem Wire-DTO.
pub(crate) const MESSAGE_TYPE: &str = "Ingested";

/// Das serialisierbare **Wire-DTO** des „Eingelesen"-Commands: die flache, serde-fähige
/// Form, die io in die Outbox schreibt und der nächste Slice mit derselben Konvention wieder
/// liest ([`outbox_payload`](shared_kernel::outbox_payload): camelCase). Die Domänen-Wert-Typen
/// der [`Line`] werden dabei auf Primitive abgeflacht (die Domäne bleibt serde-frei); rohe
/// Werte dürfen hier serialisieren (ADR-0008-Reframe: lokaler Outbox-Hop, kein Egress).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IngestedMessage {
    ordering_account: String,
    counter_account: Option<String>,
    amount_cent: u64,
    direction: &'static str,
    recipient_name: String,
    creditor_id: Option<String>,
    booking_day: String,
    value_date: String,
    purpose: Option<String>,
    booking_text: Option<String>,
}

impl IngestedMessage {
    /// Flacht die [`Line`] auf die serialisierbare Wire-Form ab (Publisher-Bridge).
    pub(crate) fn from_line(line: &Line) -> Self {
        Self {
            ordering_account: line.ordering_account.clone(),
            counter_account: line.counter_account.clone(),
            amount_cent: line.amount_cent,
            direction: match line.direction {
                Direction::Incoming => "Incoming",
                Direction::Outgoing => "Outgoing",
            },
            recipient_name: line.recipient_name.clone(),
            creditor_id: line.creditor_id.clone(),
            booking_day: as_iso(line.booking_day),
            value_date: as_iso(line.value_date),
            purpose: line.purpose.clone(),
            booking_text: line.booking_text.clone(),
        }
    }
}

/// Flacht einen Kalendertag auf die ISO-Textform `JJJJ-MM-TT` ab.
fn as_iso(day: CalendarDay) -> String {
    format!("{:04}-{:02}-{:02}", day.year, day.month, day.day)
}

impl OutboxCommand for IngestedMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.ordering_account
    }
}
