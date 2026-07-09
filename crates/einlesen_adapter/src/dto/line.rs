use einlesen_domain::LineAggregate;

use crate::abstractions::CalendarDay;

/// Die Richtung einer Buchung — **adapter-eigen** (kein Re-Export von
/// `einlesen_domain::Direction`): dieselbe Zwei-Wert-Unterscheidung wie die Domäne, aber ein
/// eigener Typ, weil `Line` die Adapter-Grenze überquert und keine Domänen-Interna dorthin
/// durchreichen soll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Incoming,
    Outgoing,
}

/// Die Währung eines Betrags — **adapter-eigen** (kein Re-Export von
/// `einlesen_domain::Currency`): derselbe Summentyp wie die Domäne (heute nur `Euro`), aber
/// ein eigener Typ, weil `Line` die Adapter-Grenze überquert und keine Domänen-Interna
/// dorthin durchreichen soll. Sie reist **neben** dem Cent-Betrag mit — ein Betrag ohne
/// Währung wäre stromabwärts nicht deutbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Euro,
}

/// Das ausgehende DTO des Slice — die **eine** Datensicht, die `einlesen` nach außen gibt (als
/// Nutzlast des „Eingelesen"-Commands). Es spiegelt das `LineAggregate` der Domäne, ohne einen
/// Domänentyp über die Adapter-Grenze zu lassen: die rohen Identifikatoren (IBANs, Klarname)
/// reisen hier **roh** — das Obfuskieren ist der nachgelagerte `obfuskieren`-Slice
/// ([ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)-Reframe: rohe
/// Werte dürfen lokal durch die Outbox, kein Egress; darum bewusst **kein** `Serialize`-Guard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub ordering_account: String,
    pub counter_account: Option<String>,
    pub amount_cent: u64,
    pub currency: Currency,
    pub direction: Direction,
    pub recipient_name: String,
    pub creditor_id: Option<String>,
    pub booking_day: CalendarDay,
    pub value_date: CalendarDay,
    pub purpose: Option<String>,
    pub booking_text: Option<String>,
}

impl Line {
    /// Projiziert das validierte Aggregat auf das DTO — die einzige Stelle, an der ein
    /// `LineAggregate` zu einer `Line` wird (der Handler ruft sie je gültiger Zeile).
    pub(crate) fn from_aggregate(aggregate: &LineAggregate) -> Self {
        let recipient = aggregate.recipient();
        let booking_day = aggregate.date().booking_day();
        let value_date = aggregate.date().value_date();
        Self {
            ordering_account: aggregate.ordering_account().as_str().to_string(),
            counter_account: aggregate
                .counter_account()
                .map(|iban| iban.as_str().to_string()),
            amount_cent: aggregate.amount().cent(),
            currency: match aggregate.amount().currency() {
                einlesen_domain::Currency::Euro => Currency::Euro,
            },
            direction: match aggregate.direction() {
                einlesen_domain::Direction::Incoming => Direction::Incoming,
                einlesen_domain::Direction::Outgoing => Direction::Outgoing,
            },
            recipient_name: recipient.name().as_str().to_string(),
            creditor_id: recipient.creditor_id().map(|id| id.as_str().to_string()),
            booking_day: CalendarDay {
                year: booking_day.year(),
                month: booking_day.month(),
                day: booking_day.day(),
            },
            value_date: CalendarDay {
                year: value_date.year(),
                month: value_date.month(),
                day: value_date.day(),
            },
            purpose: aggregate
                .purpose()
                .map(|purpose| purpose.as_str().to_string()),
            booking_text: aggregate
                .booking_text()
                .map(|text| text.as_str().to_string()),
        }
    }
}
