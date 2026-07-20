//! Die **spec-lokalen** Fixtures/Helfer des auswerten-Slice — Input-Builder, Expected-Builder und
//! Test-Consts, die die nackten Tests in `tests/` per Modul-Include (`#[path = …] mod fixtures;`)
//! hereinziehen. Erwartungsdaten sind test-seitig und gehören darum hierher, nicht in die
//! Adapter-`TestApi`.

use auswerten_adapter::{
    AnalysisWindowRow, BookingDirection, BookingRecipient, CounterAccountRow, EvaluationTestApi,
    MonthlyCashflowRow, OwnAccountRow, OwnAccountsConfig, WindowBookingRow,
};

/// Der **opake Seed-Marker** des Normalfalls: Fenster und Eigene-Konten-Config sind mit **demselben**
/// Seed tokenisiert, also tragen beide dieses Etikett. Es ist **nicht** der Seed — nur ein nicht
/// umkehrbares Kennzeichen seiner Identität, das io beiden Seiten mitgibt (ADR-0010: der Seed selbst
/// erreicht diesen Slice nie).
pub(crate) const SEED_MARKER: &str = "marker-a";

/// Die opake ID des eingefrorenen Auswertungsfensters, auf die das konsumierte
/// `FensterGeschlossen` zeigt — die **Ablage von `fenstern`** hat sie vergeben (uniformes
/// ID-Handoff, ADR-0012: Message = Zeiger, Tabelle = Wahrheit).
pub(crate) const FENSTER: &str = "fenster-1";

// Buchungstage als sortierbare Tag-Stamps (Tage seit Epoche, ADR-0013 Entscheidung C). Die
// Kalender-Sicht dahinter steht im Namen — genau sie rekonstruiert dieser Slice beim Bucketing.
/// 2024-01-15
pub(crate) const JAN_15: i64 = 19_737;
/// 2024-01-31 — der letzte Tag des Januar (die Monatsgrenze von unten).
pub(crate) const JAN_31: i64 = 19_753;
/// 2024-02-01 — der erste Tag des Februar (die Monatsgrenze von oben).
pub(crate) const FEB_01: i64 = 19_754;
/// 2024-02-29 — Schalttag.
pub(crate) const FEB_29: i64 = 19_782;
/// 2024-03-05
pub(crate) const MAR_05: i64 = 19_787;

/// Eine TestApi mit leerer Fenster-Ablage und **leerer** Eigene-Konten-Config unter dem
/// Normalfall-Seed-Marker — die Lage „ich führe keine eigenen Konten", in der jedes Gegenkonto
/// fremd ist. Wer die Eigen-Menge braucht, arrangiert sie mit `with_own_accounts`.
pub(crate) fn api() -> EvaluationTestApi {
    EvaluationTestApi::new().with_own_accounts(eigene_konten(&[]))
}

/// Ein eingefrorenes Auswertungsfenster in Euro, wie der Lese-Seam es zur ID liefert — unter dem
/// **Normalfall-Seed-Marker**.
pub(crate) fn fenster(bookings: &[WindowBookingRow]) -> AnalysisWindowRow {
    fenster_mit_marker(SEED_MARKER, bookings)
}

/// Dasselbe Fenster unter einem **frei gewählten** Seed-Marker — für den Fall, dass Fenster und
/// Config aus verschiedenen Tokenisierungen stammen.
pub(crate) fn fenster_mit_marker(
    seed_marker: &str,
    bookings: &[WindowBookingRow],
) -> AnalysisWindowRow {
    AnalysisWindowRow {
        currency: "EUR".to_string(),
        seed_marker: seed_marker.to_string(),
        bookings: bookings.to_vec(),
    }
}

/// Ein **Abfluss** an einem Buchungstag (Betrag vorzeichenlos in Cent — das Vorzeichen lebt allein
/// in der Richtung, ADR-0005) — **ohne Gegenpartei**, wie eine Entgeltzeile.
pub(crate) fn ausgang(booking_day: i64, cent: u64) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Outgoing,
        ..WindowBookingRow::default()
    }
}

/// Ein **Zufluss** an einem Buchungstag — ebenfalls ohne Gegenpartei.
pub(crate) fn eingang(booking_day: i64, cent: u64) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Incoming,
        ..WindowBookingRow::default()
    }
}

/// Ein Abfluss an eine **sichere Firma** — so, wie `obfuskieren` sie ablegt: die Gläubiger-ID als
/// öffentlicher Anker, der Firmenname im Klartext (bewusstes Kategorie-Signal) und der
/// Verwendungszweck als obfuskierter Freitext.
pub(crate) fn firma(
    booking_day: i64,
    cent: u64,
    creditor_id: &str,
    name: &str,
    purpose: Option<&str>,
) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Outgoing,
        recipient: BookingRecipient::Company {
            name: Some(name.to_string()),
            creditor_id: creditor_id.to_string(),
        },
        purpose: purpose.map(str::to_string),
        counter_account: None,
    }
}

/// Ein Abfluss an eine Firma **ohne geführten Namen** — erreichbar, weil `recipient_name` von
/// `einlesen` an durchweg optional ist, die Gläubiger-ID an einer Firma dagegen Pflicht.
pub(crate) fn firma_ohne_namen(
    booking_day: i64,
    cent: u64,
    creditor_id: &str,
    purpose: Option<&str>,
) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Outgoing,
        recipient: BookingRecipient::Company {
            name: None,
            creditor_id: creditor_id.to_string(),
        },
        purpose: purpose.map(str::to_string),
        counter_account: None,
    }
}

/// Ein Abfluss an eine **fail-safe Privatperson** — sie trägt **nur** ihr seed-basiertes Pseudonym
/// (`PERSON_…`), das `obfuskieren` vergeben hat. Ein anderer Seed ergibt stromaufwärts ein anderes
/// Pseudonym; dieser Slice sieht den Seed nie (ADR-0010).
pub(crate) fn person(booking_day: i64, cent: u64, pseudonym: &str) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Outgoing,
        recipient: BookingRecipient::Person {
            pseudonym: pseudonym.to_string(),
        },
        purpose: None,
        counter_account: None,
    }
}

/// Ein Abfluss auf ein **Gegenkonto**, das nur als Token vorliegt (`KONTO_…`) — so, wie
/// `obfuskieren` es an der Datenschutz-Grenze abgelegt hat. Genau dieses Token prüft die Auflösung
/// gegen die Eigen-Menge; eine rohe IBAN erreicht diesen Slice nie (ADR-0013, Entscheidung F).
pub(crate) fn gegenkonto(booking_day: i64, cent: u64, token: &str) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_cent: cent,
        direction: BookingDirection::Outgoing,
        counter_account: Some(token.to_string()),
        ..WindowBookingRow::default()
    }
}

/// Die **token-orientierte Eigene-Konten-Config**, wie sie dem Handler als **Wert** hereingereicht
/// wird: je eigenem Konto sein `KONTO_…`-Token (vom Loader mit **demselben** Seed tokenisiert wie
/// die Gegenkonto-Token) und seine Rolle. Erfundene Werte — die echte Config bleibt HITL (0002).
pub(crate) fn eigene_konten(accounts: &[(&str, &str)]) -> OwnAccountsConfig {
    eigene_konten_mit_marker(SEED_MARKER, accounts)
}

/// Dieselbe Config unter einem **frei gewählten** Seed-Marker — so lässt sich eine Eigen-Menge
/// arrangieren, die aus einer **anderen** Tokenisierung stammt als das Fenster.
pub(crate) fn eigene_konten_mit_marker(
    seed_marker: &str,
    accounts: &[(&str, &str)],
) -> OwnAccountsConfig {
    OwnAccountsConfig {
        seed_marker: seed_marker.to_string(),
        accounts: accounts
            .iter()
            .map(|(token, role)| OwnAccountRow {
                token: (*token).to_string(),
                role: (*role).to_string(),
            })
            .collect(),
    }
}

/// Ein erwartetes **eigenes** Gegenkonto samt seiner Rolle — die Rolle trägt die Deutung (ein
/// Zufluss aufs Spar-Konto ist Sparen, nicht Konsum).
pub(crate) fn eigen(role: &str) -> CounterAccountRow {
    CounterAccountRow::Own {
        role: role.to_string(),
    }
}

/// Ein erwarteter **Monats-Bucket** des Cashflows: Zu- und Abfluss getrennt und je vorzeichenlos.
pub(crate) fn monat(year: i64, month: i64, incoming: u64, outgoing: u64) -> MonthlyCashflowRow {
    MonthlyCashflowRow {
        year,
        month,
        incoming_cent: incoming,
        outgoing_cent: outgoing,
    }
}
