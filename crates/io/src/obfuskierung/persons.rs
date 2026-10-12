//! Die reale io-Erfüllung des öffentlichen [`KnownPersonsSeam`] des `obfuskierung`-Adapters
//! ([Issue 0010](../../../../docs/issues/0010-scrubbing-regeln-fuer-obfuskiertertext.md)): die
//! **Namensliste** des Freitext-Engpasses. Sie führt zwei Quellen zusammen:
//!
//! 1. **Errechnet:** alle Empfänger-Namen aus dem einlesen-Bestand (`einlesen_lines.recipient_name`),
//!    die **keine** Gläubiger-ID tragen — genau die Gegenparteien, die der Slice fail-safe als
//!    Person deutet. Firmen **mit** Gläubiger-ID bleiben draußen: ihr Name ist das wichtigste
//!    Kategorie-Signal und würde sonst in jedem Verwendungszweck mitgeschwärzt. Der bekannte Preis:
//!    eine Firma **ohne** Gläubiger-ID landet auf der Liste und fällt im Freitext mit.
//! 2. **Eingegeben:** die lokale `personen.toml` ([`PersonenConfig`](crate::config::PersonenConfig))
//!    für Namen, die nie Empfänger waren, und für Teilnamen von Empfängern (Vornamen, Kurzformen),
//!    die der Engpass nicht von selbst aus dem vollständigen Namen ableitet.
//!
//! **Nur Gebuchtes** ([ADR-0016](../../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
//! eine **vorgemerkte** Zeile erreicht `einlesen_lines` nie — die ACL des einlesen-Slice verwirft sie
//! vor jeder Prüfung. Ihr Empfänger-Name steht darum erst auf der Liste, wenn die Zeile gebucht in
//! einem späteren Auszug ankommt.
//!
//! **Je Buchung gelesen**, nicht einmal je Lauf: die einlesen-Zeilen des laufenden Imports liegen
//! erst nach dem Import-Schritt vor, und ihre Namen sollen schon in derselben Obfuskierung zählen.
//!
//! **Reihenfolge-Abhängigkeit — Namen wirken nicht rückwirkend.** Eine Buchung wird mit genau den
//! Namen gescrubbt, die bei ihrer Obfuskierung bekannt sind. Taucht ein Empfänger erst in einem
//! **späteren** Import auf, bleibt sein Name in schon obfuskierten Verwendungszwecken früherer Läufe
//! stehen; ein Re-Scrub des Bestands gibt es bewusst nicht. Bedienregel: **vor dem ersten Lauf** alle
//! bekannten Empfänger-Namen aller Konten (samt Vornamen/Kurzformen) in die `personen.toml` eintragen.
//! Gelesen wird **außerhalb** der committenden Transaktion — wie die einlesen-Zeile selbst ist der
//! Bestand append-only.
//!
//! **Keine Klartext↔Token-Zuordnung** (ADR-0010): nur Namen, kein Token, kein Seed; nichts davon
//! wird abgelegt. **Redigiert:** ein Fehlschlag nennt die Beschaffungs-Mechanik, nie einen Namen.

use obfuskierung_adapter::{KnownPersonsSeam, PersonsUnavailable};

use crate::sqlite::SqliteOutbox;

/// Der reale Namens-Seam: Empfänger-Namen ohne Gläubiger-ID aus dem einlesen-Bestand plus die
/// konfigurierten Namen. `Debug` nennt nur die Zahl der konfigurierten Namen.
#[derive(Clone)]
pub struct KnownPersonNames {
    outbox: SqliteOutbox,
    configured: Vec<String>,
}

impl std::fmt::Debug for KnownPersonNames {
    /// Redigiert: nur die Anzahl der konfigurierten Namen, nie ein Name.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnownPersonNames")
            .field("configured", &self.configured.len())
            .finish_non_exhaustive()
    }
}

impl KnownPersonNames {
    /// Bindet den Seam an das SQLite-Rückgrat (auf dem der `einlesen`-Store schreibt) und an die
    /// konfigurierten Namen. Legt **nichts** an — die `einlesen_lines`-Tabelle gehört dem
    /// `einlesen`-Store.
    pub fn new(outbox: SqliteOutbox, configured: Vec<String>) -> Self {
        Self { outbox, configured }
    }
}

impl KnownPersonsSeam for KnownPersonNames {
    // ponytail: liest je Buchung die **ganze** Namensliste — über einen Lauf mit N Buchungen also
    // O(N²) Zeilen. Bei ein paar tausend Buchungen eines Privatkontos sind das Millisekunden; wird es
    // spürbar, ist der Ausweg, die Liste **einmal je Lauf** nach dem Import zu lesen (die Namen des
    // laufenden Imports liegen dann schon vor) und als Wert durchzureichen.
    fn known_persons(&self) -> Result<Vec<String>, PersonsUnavailable> {
        let recipients = self
            .outbox
            .with_connection(|conn| {
                let mut statement = conn.prepare(SELECT_PERSON_NAMES)?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()
            })
            .map_err(|err| PersonsUnavailable {
                message: format!("Namensliste nicht lesbar: {err}"),
            })?;
        Ok(recipients
            .into_iter()
            .chain(self.configured.iter().cloned())
            .collect())
    }
}

/// Die Empfänger-Namen, die der Slice als Person deutet: ohne Gläubiger-ID, nicht leer, je einmal.
const SELECT_PERSON_NAMES: &str = "\
SELECT DISTINCT recipient_name
FROM einlesen_lines
WHERE recipient_name IS NOT NULL
  AND trim(recipient_name) <> ''
  AND (creditor_id IS NULL OR trim(creditor_id) = '');";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::einlesen::EinlesenSqliteStore;

    /// Legt eine einlesen-Zeile mit Empfänger und optionaler Gläubiger-ID direkt an — nur die
    /// Spalten, auf die es hier ankommt; der Rest sind erfundene Pflichtwerte.
    fn zeile(
        outbox: &SqliteOutbox,
        id: &str,
        empfaenger: &str,
        glaeubiger_id: Option<&str>,
    ) -> rusqlite::Result<()> {
        outbox.with_connection(|conn| {
            conn.execute(
                "INSERT INTO einlesen_lines (
                     id, row_key, ordering_account, amount_minor_units, currency, direction,
                     recipient_name, creditor_id,
                     booking_year, booking_month, booking_day, value_year, value_month, value_day
                 ) VALUES (?1, 'schluessel-' || ?1, 'DE00000000000000000000', 100, 'EUR',
                           'outgoing', ?2, ?3, 2026, 7, 1, 2026, 7, 1);",
                rusqlite::params![id, empfaenger, glaeubiger_id],
            )
            .map(|_rows| ())
        })
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert_eq! ist die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn liefert_empfaenger_ohne_glaeubiger_id_und_die_konfigurierten_namen() -> rusqlite::Result<()>
    {
        let outbox = SqliteOutbox::open_in_memory()?;
        EinlesenSqliteStore::with_own_tables(outbox.clone())?;
        zeile(&outbox, "1", "LISA BEISPIEL", None)?;
        zeile(&outbox, "2", "LISA BEISPIEL", None)?;
        zeile(
            &outbox,
            "3",
            "STROMWERK FIKTIV AG",
            Some("DE00ZZZ00000000007"),
        )?;
        let seam = KnownPersonNames::new(outbox, vec!["Oma Fiktiv".to_string()]);

        let namen = seam.known_persons().map_err(|_unavailable| {
            rusqlite::Error::InvalidParameterName("Namensliste nicht lesbar".to_string())
        })?;

        // Die Person je einmal, die Firma mit Gläubiger-ID nicht, der konfigurierte Name dazu.
        assert_eq!(namen, vec!["LISA BEISPIEL", "Oma Fiktiv"]);
        Ok(())
    }

    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn ohne_einlesen_tabelle_scheitert_die_namensliste_terminal_und_redigiert()
    -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        // Bewusst **keine** einlesen-Tabelle angelegt.
        let seam = KnownPersonNames::new(outbox, vec!["Oma Fiktiv".to_string()]);

        let fehler = seam
            .known_persons()
            .expect_err("ohne einlesen-Tabelle muss die Namensliste terminal scheitern");

        // Positiv das Diagnose-Präfix, negativ kein Name — auch nicht der konfigurierte.
        assert!(fehler.message.starts_with("Namensliste nicht lesbar"));
        assert!(!fehler.message.contains("Oma Fiktiv"));
        Ok(())
    }
}
