//! Was der Bericht neben dem fenster-übergreifenden Ergebnis aus der **Datei** liest: allein die
//! **Firmennamen** — gescopt über die `run_id`, die der eigene Lauf gemintet hat (ADR-0016), nie über
//! `max(run_id)`. Jede **Zahl** des Berichts kommt aus dem Ergebnis selbst, nicht von hier.
//!
//! Der Name einer Firma steht gescrubbt in `obfuskiert` (er ist dort ein bewusster Analyse-Anker).
//! Gelesen werden nur Firmen der Fenster dieses Laufs; trägt dieselbe Gläubiger-ID mehrere
//! Schreibweisen, gilt die der jüngsten Buchung.

use std::collections::BTreeMap;

use rusqlite::{Connection, params};

/// Gläubiger-ID → gescrubbter Firmenname der Buchungen des Laufs `run_id`.
pub(crate) fn company_names(
    conn: &Connection,
    run_id: &str,
) -> rusqlite::Result<BTreeMap<String, String>> {
    conn.prepare(SELECT_COMPANY_NAMES)?
        .query_map(params![run_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        // Die Abfrage liefert die jüngste Buchung zuerst; die erste Schreibweise je ID gewinnt.
        .try_fold(BTreeMap::new(), |mut names, entry| {
            let (creditor_id, name) = entry?;
            names.entry(creditor_id).or_insert(name);
            Ok(names)
        })
}

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
