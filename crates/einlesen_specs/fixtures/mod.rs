//! Die **spec-lokalen** Fixtures/Helfer des einlesen-Slice — Input-Builder, Expected-Builder
//! und Test-Consts, die die nackten Tests in `tests/` per Modul-Include (`#[path = …] mod
//! fixtures;`) hereinziehen. Bewusst hier statt in der Adapter-`TestApi`: `RawRecord` trägt
//! keine Test-Ergonomie, und die Expected-Builder sind test-seitige Erwartungsdaten (die
//! erwartete Projektion eines Laufs), kein Teil der Adapter-API.

use einlesen_adapter::{CalendarDay, Currency, Direction, Line, RawRecord};

/// Die Quelle eines Einlese-Laufs — seit Issue 0018 reiner Routing-Kontext: sie reist nur
/// noch als `partition_key` des Envelopes (die Payload ist feldlos), der Batch kommt über
/// den Quell-Seam.
pub(crate) const QUELLE: &str = "auszug-juni";

pub(crate) fn feld(wert: &str) -> Option<String> {
    Some(wert.to_string())
}

/// Ein vollständig gültiger Rohsatz — spec-lokale Fixture; Feld-Overrides passieren direkt
/// (die Felder sind `pub`, kein Fluent-Bauer nötig).
pub(crate) fn gültiger_rohsatz() -> RawRecord {
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

/// Die erwartete Projektion des [`gültiger_rohsatz`] auf das ausgehende `Line`-DTO — spec-lokale
/// Fixture, damit die Feld-Treue-Specs eine **ganze** `Line` vergleichen (`assert_eq!`), statt
/// Feld für Feld zu assertieren. Die `Line`-Felder sind `pub`, ein Struct-Literal genügt.
pub(crate) fn erwartete_zeile() -> Line {
    Line {
        ordering_account: "DE89370400440532013000".to_string(),
        counter_account: Some("DE11520513735120710131".to_string()),
        amount_cent: 5_000,
        currency: Currency::Euro,
        direction: Direction::Outgoing,
        recipient_name: Some("EDEKA SAGT DANKE".to_string()),
        creditor_id: Some("DE98ZZZ09999999999".to_string()),
        booking_day: CalendarDay {
            year: 2024,
            month: 6,
            day: 13,
        },
        value_date: CalendarDay {
            year: 2024,
            month: 6,
            day: 14,
        },
        purpose: Some("Einkauf".to_string()),
        booking_text: Some("LASTSCHRIFT".to_string()),
    }
}
