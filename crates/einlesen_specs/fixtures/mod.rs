//! Die **spec-lokalen** Fixtures/Helfer des einlesen-Slice — Input-Builder, Expected-Builder
//! und Test-Consts, die die nackten Tests in `tests/` per Modul-Include (`#[path = …] mod
//! fixtures;`) hereinziehen. Bewusst hier statt in der Adapter-`TestApi`: `RawRecord` trägt
//! keine Test-Ergonomie, und die Expected-Builder sind test-seitige Erwartungsdaten (die
//! erwartete Projektion eines Laufs), kein Teil der Adapter-API.

use einlesen_adapter::{CalendarDay, Direction, Line, RawRecord, RecordStatus};

/// Die Quelle eines Einlese-Laufs — seit Issue 0018 reiner Routing-Kontext: sie reist nur
/// noch als `partition_key` des Envelopes (die Payload ist feldlos), der Batch kommt über
/// den Quell-Seam.
pub(crate) const QUELLE: &str = "auszug-juni";

/// Der Zeilen-Schlüssel des [`gültiger_rohsatz`] — ein erfundener, opaker Wert, wie ihn die Quelle
/// als Digest vergäbe (ADR-0016). Specs mit mehreren Zeilen geben jeder Zeile über
/// [`rohsatz_mit_schlüssel`] einen eigenen.
pub(crate) const SCHLÜSSEL: &str = "zeile-1";

/// Der [`gültiger_rohsatz`] unter einem **eigenen** Zeilen-Schlüssel — für Specs, deren Zeilen
/// verschiedene Buchungen sind (gleiche Schlüssel hießen: dieselbe Zeile).
pub(crate) fn rohsatz_mit_schlüssel(schlüssel: &str) -> RawRecord {
    RawRecord {
        row_key: feld(schlüssel),
        ..gültiger_rohsatz()
    }
}

/// Eine **vorgemerkte** Zeile, wie die Bank sie exportiert: leere Valuta, `0` als IBAN, keine
/// Kundenreferenz und darum (in der echten Quelle) auch kein eigener Schlüssel-Bestandteil — sie
/// überspränge sonst die Pflichtfeld-Prüfung nicht. Erfundene Werte.
pub(crate) fn vorgemerkter_rohsatz() -> RawRecord {
    RawRecord {
        row_key: feld("vorgemerkt-1"),
        status: RecordStatus::Pending,
        value_date: None,
        counter_account: feld("0"),
        amount: feld("-12,34"),
        ..gültiger_rohsatz()
    }
}

pub(crate) fn feld(wert: &str) -> Option<String> {
    Some(wert.to_string())
}

/// Ein vollständig gültiger Rohsatz — spec-lokale Fixture; Feld-Overrides passieren direkt
/// (die Felder sind `pub`, kein Fluent-Bauer nötig).
pub(crate) fn gültiger_rohsatz() -> RawRecord {
    RawRecord {
        row_key: feld(SCHLÜSSEL),
        status: RecordStatus::Booked,
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
        row_key: SCHLÜSSEL.to_string(),
        ordering_account: "DE89370400440532013000".to_string(),
        counter_account: Some("DE11520513735120710131".to_string()),
        amount_minor_units: 5_000,
        currency: "EUR".to_string(),
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
