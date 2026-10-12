//! Was der Bericht neben dem fenster-übergreifenden Ergebnis aus der **Datei** liest — gescopt über
//! die `run_id`, die der eigene Lauf gemintet hat (ADR-0016), nie über `max(run_id)`.
//!
//! - **Umbuchungen** und **Gebühren** liegen je Fenster in `window_evaluation_transfers` bzw.
//!   `window_evaluation_fees`. Die Fenster eines Laufs zerlegen seine Zeitachse ohne Überlappung,
//!   ein Monat kann aber in zwei Fenstern liegen; die Abfrage summiert darum je Monat und
//!   Gegenkonto-Rolle bzw. belastetem Konto — die Domäne hat jedes Teilstück schon errechnet.
//! - **Firmennamen** stehen gescrubbt in `obfuskiert` (der Name einer Firma ist ein bewusster
//!   Analyse-Anker). Gelesen werden nur Firmen der Fenster dieses Laufs; trägt dieselbe Gläubiger-ID
//!   mehrere Schreibweisen, gilt die der jüngsten Buchung.

use std::collections::BTreeMap;

use auswerten_adapter::{FeeRow, InternalTransferRow};
use rusqlite::{Connection, Row, params};

/// Was der Bericht aus der Datei über **einen** Lauf liest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RunDetails {
    pub(crate) transfers: Vec<InternalTransferRow>,
    pub(crate) fees: Vec<FeeRow>,
    pub(crate) company_names: BTreeMap<String, String>,
}

/// Liest Umbuchungen, Gebühren und Firmennamen des Laufs `run_id`.
pub(crate) fn read_run(conn: &Connection, run_id: &str) -> rusqlite::Result<RunDetails> {
    Ok(RunDetails {
        transfers: conn
            .prepare(SELECT_TRANSFERS)?
            .query_map(params![run_id], |row| {
                Ok(InternalTransferRow {
                    year: row.get(0)?,
                    month: row.get(1)?,
                    role: row.get(2)?,
                    incoming_minor_units: minor_units(row, 3)?,
                    outgoing_minor_units: minor_units(row, 4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?,
        fees: conn
            .prepare(SELECT_FEES)?
            .query_map(params![run_id], |row| {
                Ok(FeeRow {
                    year: row.get(0)?,
                    month: row.get(1)?,
                    account: row.get(2)?,
                    incoming_minor_units: minor_units(row, 3)?,
                    outgoing_minor_units: minor_units(row, 4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?,
        company_names: conn
            .prepare(SELECT_COMPANY_NAMES)?
            .query_map(params![run_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            // Die Abfrage liefert die jüngste Buchung zuerst; die erste Schreibweise je ID gewinnt.
            .try_fold(BTreeMap::new(), |mut names, entry| {
                let (creditor_id, name) = entry?;
                names.entry(creditor_id).or_insert(name);
                Ok::<_, rusqlite::Error>(names)
            })?,
    })
}

/// Eine Betrags-Summe — abgelegt sind nur vorzeichenlose Beträge, ein negativer Wert ist ein Bruch
/// der Datei und wird nicht still umgedeutet.
fn minor_units(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|_negative| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

const SELECT_TRANSFERS: &str = "\
SELECT t.year, t.month, t.role, sum(t.incoming_minor_units), sum(t.outgoing_minor_units)
FROM auswertungsfenster w
JOIN window_evaluations e ON e.window_id = w.id
JOIN window_evaluation_transfers t ON t.evaluation_id = e.id
WHERE w.run_id = ?1
GROUP BY t.year, t.month, t.role
ORDER BY t.year, t.month, t.role;";

const SELECT_FEES: &str = "\
SELECT f.year, f.month, f.account, sum(f.incoming_minor_units), sum(f.outgoing_minor_units)
FROM auswertungsfenster w
JOIN window_evaluations e ON e.window_id = w.id
JOIN window_evaluation_fees f ON f.evaluation_id = e.id
WHERE w.run_id = ?1
GROUP BY f.year, f.month, f.account
ORDER BY f.year, f.month, f.account;";

const SELECT_COMPANY_NAMES: &str = "\
SELECT o.recipient_creditor_id, o.recipient_name
FROM auswertungsfenster w
JOIN auswertungsfenster_buchungen b ON b.window_id = w.id
JOIN obfuskiert o ON o.id = b.booking_id
WHERE w.run_id = ?1
  AND o.recipient_kind = 'company'
  AND o.recipient_creditor_id IS NOT NULL
  AND o.recipient_name IS NOT NULL
ORDER BY o.booking_day DESC, o.id;";
