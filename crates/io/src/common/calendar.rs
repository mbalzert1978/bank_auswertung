//! Die **eine** Umrechnung zwischen dem gespeicherten `YYYY-MM-DD`-Textstamp und dem `i64`-Tag-Stamp
//! der Pipeline-Zeitachse — geteilt von allen io-Lese-Seams, die `obfuskiert.booking_day` bzw.
//! `auswertungsfenster.from_day`/`to_day` berühren (`fenstern` und `auswerten`).
//!
//! **Konvention: Tage seit der Unix-Epoche (1970-01-01).** Das ist **keine** freie Wahl, sondern die
//! Achse, die die Domäne definiert: `auswerten_domain`s Kalender-Rekonstruktion (`civil_year_month`,
//! Howard Hinnants `civil_from_days`) verschiebt den Nullpunkt um `719_468` Tage auf den 1. März 0000
//! **relativ zur Unix-Epoche**. Deutete io den Stamp anders (etwa als Tage seit Christi Geburt), fiele
//! die rekonstruierte Kalender-Sicht um ~1969 Jahre daneben. `fenstern` rechnet auf derselben Achse nur
//! `± Tage` (konventions-agnostisch) — aber sie **teilt** sie mit `auswerten` (ADR-0013, Entscheidung C:
//! „dieselbe Achse"), also muss io durchgehend die Domänen-Konvention treffen.
//!
//! **Lexikografische Sortierung bleibt unberührt:** die SQL-Statements sortieren/vergleichen auf der
//! `TEXT`-Spalte (`MAX(booking_day)`, `booking_day < ?`), nicht auf dem `i64` — die Wahl der Epoche
//! ändert daran nichts. Der `i64` trägt allein die **arithmetische** und die **Kalender**-Sicht.
//!
//! **Die Achse stand fest, bevor die erste durable Zeile geschrieben wurde.** Die Umstellung von der
//! früheren CE-Zählung (`num_days_from_ce`) auf diese Achse ändert die Bedeutung persistierter
//! `INTEGER`-Stamps (`auswertungsfenster.from_day`/`to_day`). Sie geschah, bevor es ein Binary und
//! damit eine durable Datei gab — eine DB auf der alten Achse existiert nicht.
//!
//! **Eine künftige Achsen-Änderung ist eine Migration.** Seit Issue 0084 öffnet der Lauf seine Datei
//! über [`schema::open`](crate::schema::open): eine frische Datei wird mit der Schema-Version gestempelt
//! (`PRAGMA user_version`), eine abweichende abgewiesen. Ändert sich die Achse wieder, zählt die
//! Version hoch und `schema::ensure` bekommt den Migrations-Arm — fail-fast statt still korrumpiert.

use chrono::{Datelike, NaiveDate};

/// `num_days_from_ce` des Unix-Epoche-Tags (1970-01-01) — der Offset zwischen chronos CE-Zählung
/// (0001-01-01 = 1) und der Domänen-Achse (1970-01-01 = 0). Der Test `unix_epoche_ist_null` nagelt
/// ihn fest, damit ein Zahlendreher nicht still die ganze Zeitachse verschiebt.
const CE_DAYS_AT_UNIX_EPOCH: i32 = 719_163;

/// Deutet einen gespeicherten `YYYY-MM-DD`-Textstamp zum sortierbaren `i64`-Tag-Stamp (Tage seit der
/// Unix-Epoche). `None`, wenn der Text kein gültiges ISO-Datum ist.
pub(crate) fn day_stamp(iso: &str) -> Option<i64> {
    NaiveDate::parse_from_str(iso, "%Y-%m-%d")
        .ok()
        .map(|date| i64::from(date.num_days_from_ce() - CE_DAYS_AT_UNIX_EPOCH))
}

/// Setzt den `i64`-Tag-Stamp zurück zum kanonischen `YYYY-MM-DD`-Textstamp (das Gegenstück zu
/// [`day_stamp`]). `None`, wenn der Stamp jenseits des von chrono darstellbaren Kalenderbereichs liegt
/// (praktisch unerreichbar: er stammt stets aus einem zuvor per [`day_stamp`] gelesenen Kalendertag).
pub(crate) fn iso_day(stamp: i64) -> Option<String> {
    date(stamp).map(|date| date.format("%Y-%m-%d").to_string())
}

/// Der `i64`-Tag-Stamp in **deutscher Schreibweise** (`TT.MM.JJJJ`) — für den Bericht (Issue 0012).
/// `None` unter derselben Bedingung wie bei [`iso_day`].
pub(crate) fn german_day(stamp: i64) -> Option<String> {
    date(stamp).map(|date| date.format("%d.%m.%Y").to_string())
}

fn date(stamp: i64) -> Option<NaiveDate> {
    stamp
        .checked_add(i64::from(CE_DAYS_AT_UNIX_EPOCH))
        .and_then(|ce| i32::try_from(ce).ok())
        .and_then(NaiveDate::from_num_days_from_ce_opt)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Der Unix-Epoche-Tag ist der Nullpunkt der Achse — der Test, der [`CE_DAYS_AT_UNIX_EPOCH`]
    /// festnagelt.
    #[test]
    fn unix_epoche_ist_null() {
        assert_eq!(day_stamp("1970-01-01"), Some(0));
    }

    /// Hin und zurück ist die Identität — für einen echten Kalendertag.
    #[test]
    fn hin_und_zurueck_ist_die_identitaet() {
        let stamp = day_stamp("2024-06-20").expect("gültiges ISO-Datum");
        assert_eq!(iso_day(stamp).as_deref(), Some("2024-06-20"));
    }

    /// Ein Nicht-Datum wird abgelehnt, nicht geraten.
    #[test]
    fn ein_unlesbarer_text_ist_none() {
        assert_eq!(day_stamp("kein-datum"), None);
    }
}
