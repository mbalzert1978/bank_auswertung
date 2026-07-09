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

#[cfg(test)]
mod tests {
    use crate::abstractions::{CalendarDay as AdapterKalendertag, CalendarSeam};

    use super::{CalendarDay, Direction, Line, LineAggregate};
    use crate::dto::RawRecord;

    fn feld(wert: &str) -> Option<String> {
        Some(wert.to_string())
    }

    /// Ein vollständig gültiger Rohsatz für die Mapping-Prüfung — direkter Feld-Bauer statt
    /// Fluent-Overrides (die gehören in die Specs, nicht hierher).
    fn gültiger_rohsatz() -> RawRecord {
        RawRecord {
            ordering_account: feld("DE89370400440532013000"),
            booking_day: feld("13.06.2024"),
            value_date: feld("14.06.2024"),
            amount: feld("-50,00"),
            currency: feld("EUR"),
            recipient_name: feld("EDEKA SAGT DANKE"),
            creditor_id: feld("DE98ZZZ09999999999"),
            counter_account: feld("DE11520513735120710131"),
            purpose: feld("Einkauf"),
            booking_text: feld("LASTSCHRIFT"),
        }
    }

    fn parse_deutsches_datum(roh: &str) -> Option<AdapterKalendertag> {
        let mut teile = roh.split('.');
        let tag: u8 = teile.next()?.parse().ok()?;
        let monat: u8 = teile.next()?.parse().ok()?;
        let jahr: u16 = teile.next()?.parse().ok()?;
        Some(AdapterKalendertag {
            year: jahr,
            month: monat,
            day: tag,
        })
    }

    struct DeutscherKalenderSeam;

    impl CalendarSeam for DeutscherKalenderSeam {
        fn parse(&self, roh: &str) -> Option<AdapterKalendertag> {
            parse_deutsches_datum(roh)
        }
    }

    #[test]
    fn aus_aggregat_übernimmt_alle_felder_originalgetreu() {
        let kalender = crate::bridges::CalendarBridge {
            seam: &DeutscherKalenderSeam,
        };
        let aggregat = LineAggregate::from_raw_line(gültiger_rohsatz().to_raw_line(), &kalender)
            .expect("gültige Zeile");

        let zeile = Line::from_aggregate(&aggregat);

        assert_eq!(zeile.ordering_account, "DE89370400440532013000");
        assert_eq!(
            zeile.counter_account.as_deref(),
            Some("DE11520513735120710131")
        );
        assert_eq!(zeile.amount_cent, 5_000);
        assert_eq!(zeile.direction, Direction::Outgoing);
        assert_eq!(zeile.recipient_name, "EDEKA SAGT DANKE");
        assert_eq!(zeile.creditor_id.as_deref(), Some("DE98ZZZ09999999999"));
        assert_eq!(
            zeile.booking_day,
            CalendarDay {
                year: 2024,
                month: 6,
                day: 13
            }
        );
        assert_eq!(
            zeile.value_date,
            CalendarDay {
                year: 2024,
                month: 6,
                day: 14
            }
        );
        assert_eq!(zeile.purpose.as_deref(), Some("Einkauf"));
        assert_eq!(zeile.booking_text.as_deref(), Some("LASTSCHRIFT"));
    }

    #[test]
    fn aus_aggregat_übernimmt_fehlende_optionalfelder_als_none() {
        let kalender = crate::bridges::CalendarBridge {
            seam: &DeutscherKalenderSeam,
        };
        let mut rohsatz = gültiger_rohsatz();
        rohsatz.counter_account = None;
        rohsatz.purpose = None;
        rohsatz.booking_text = None;
        let aggregat = LineAggregate::from_raw_line(rohsatz.to_raw_line(), &kalender)
            .expect("Pflichtfelder gesetzt");

        let zeile = Line::from_aggregate(&aggregat);

        assert!(zeile.counter_account.is_none());
        assert!(zeile.purpose.is_none());
        assert!(zeile.booking_text.is_none());
    }
}
