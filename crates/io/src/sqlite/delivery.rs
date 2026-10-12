//! **Was nach einem Zustellversuch mit der Nachricht geschieht**
//! ([Issue 0085](../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md),
//! [ADR-0017](../../../../docs/adr/0017-dead-letter-als-status-spalte-der-outbox.md)): die
//! Versuchszählung und das Aussondern (Dead-Letter) des Brokers. Das beantwortet die in
//! [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
//! vertagte Frage, **wie `io` einen terminalen Fehlschlag surfacet**: es nimmt die Nachricht aus der
//! Zustellung, legt den redigierten Grund an ihre Outbox-Zeile und lässt den Lauf sie benennen.
//!
//! **Form: eine Status-Spalte in der Outbox, keine eigene Tabelle.** Eine ausgesonderte Nachricht
//! bleibt dieselbe Zeile mit `status = 'dead_letter'`; sie wandert nirgendwohin.
//!
//! **Der Slice merkt davon nichts.** Er gibt weiterhin nur seinen nackten, redigierten Domänen-Fehler
//! zurück (ADR-0012); Retry und Dead-Letter sind Broker-Mechanik und leben allein hier.
//!
//! **Zwei Schichten:** die reine Regel [`disposition`] (welcher Fehlschlag wohin) und das Verbuchen
//! an der Zeile (`settle`). Was der Broker meldet, kommt **aus der getroffenen Zeile** (`RETURNING`),
//! nicht aus einer zweiten Rechnung in Rust — trifft das `UPDATE` keine offene Zeile mehr, sagt er das.

use rusqlite::{OptionalExtension, params};

use super::SqliteOutbox;
use crate::outbox::{OutboxError, OutboxMessage};

/// Wie oft der Broker eine Nachricht höchstens zustellt, bevor ein **transienter** Fehlschlag
/// (`Retry`, ein gescheiterter Commit) sie aussondert: der erste Versuch und zwei Wiederholungen.
///
/// **Eine Konstante, keine Config-Datei** (YAGNI): der Wert ist kein Fachparameter, den Markus je Lauf
/// wählt, sondern Broker-Mechanik. Ein Commit scheitert an der Datenbank selbst (eine kurz gesperrte
/// Datei, eine volle Platte); hält das über drei Versuche an, ist es nicht mehr vorübergehend, und
/// weiteres Warten verschiebt nur die Meldung. Ein terminaler Fehlschlag zählt hier nicht — er
/// sondert sofort aus.
pub const MAX_DELIVERY_ATTEMPTS: u32 = 3;

/// Wohin ein Fehlschlag die Nachricht schickt — das Ergebnis der reinen Regel [`disposition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disposition {
    /// Offen lassen und erneut zustellen.
    Redeliver,
    /// Aus der Zustellung nehmen.
    DeadLetter,
}

/// **Die Regel**, rein und ohne Datenbank: ein **terminaler** Fehlschlag (`Warning` — der Slice
/// scheiterte; `Critical` — die Hülle trug nicht) sondert sofort aus; ein **transienter** (`Retry` —
/// der Commit scheiterte) erst, wenn mit diesem Versuch [`MAX_DELIVERY_ATTEMPTS`] erreicht sind.
/// `attempts_before` ist die Zahl der Versuche **vor** dem gerade gescheiterten.
pub(crate) const fn disposition(error: &OutboxError, attempts_before: u32) -> Disposition {
    match error {
        OutboxError::Retry { .. } if attempts_before.saturating_add(1) < MAX_DELIVERY_ATTEMPTS => {
            Disposition::Redeliver
        }
        OutboxError::Retry { .. } | OutboxError::Warning { .. } | OutboxError::Critical { .. } => {
            Disposition::DeadLetter
        }
    }
}

/// Der Ausgang **eines** Zustellversuchs, nachdem der Broker ihn verbucht hat — das, was
/// [`deliver_open`](SqliteOutbox::deliver_open) je Nachricht zurückgibt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// Verarbeitet und geackt.
    Processed,
    /// **Transient** gescheitert (`Retry`) und noch Versuche übrig: die Nachricht bleibt offen, der
    /// Versuch ist gezählt, die nächste Runde stellt sie erneut zu.
    Redelivery {
        /// Der Fehlschlag dieses Versuchs.
        error: OutboxError,
        /// Die Versuche bisher, wie die Zeile sie nach dem Zählen trägt.
        attempts: u32,
    },
    /// Aus der Zustellung genommen — sofort bei einem **terminalen** Fehlschlag (`Warning`/`Critical`),
    /// bei einem transienten nach [`MAX_DELIVERY_ATTEMPTS`] Versuchen.
    DeadLettered {
        /// Der Fehlschlag dieses Versuchs.
        error: OutboxError,
        /// Die ausgesonderte Nachricht, wie sie nun an ihrer Zeile steht
        /// ([`SqliteOutbox::dead_letters`] liefert dieselbe).
        dead_letter: DeadLetter,
    },
    /// Der Versuch scheiterte, aber die Zeile war beim Verbuchen **nicht mehr offen** — eine andere
    /// Zustellung hat sie schon erledigt (etwa ein überlappender Prozess auf derselben Datei, den der
    /// idempotente Ack als „bereits verarbeitet" meldet). Nichts wurde gezählt oder ausgesondert.
    SettledElsewhere(OutboxError),
    /// Der Versuch scheiterte, und **sein Verbuchen** scheiterte auch: die Nachricht bleibt offen und
    /// ungezählt. Der Broker stellt danach in diesem Aufruf nichts weiter zu.
    Unrecorded {
        /// Der Fehlschlag dieses Versuchs.
        error: OutboxError,
        /// Die io-Diagnose, warum er sich nicht verbuchen ließ (Zugriffs-Mechanik, nie ein Zeilenwert).
        diagnosis: String,
    },
}

/// Eine **ausgesonderte** Nachricht, wie sie abgefragt wird: ihr Dispatch-Key, wie viele Versuche sie
/// bekam, und der Grund.
///
/// **Redigiert:** der Grund ist die `Display` des [`OutboxError`] — beim Slice-Fehler dessen nackter,
/// redigierter Meldungssatz (ADR-0012: statischer Feldname, opake ID oder io-Diagnose, nie ein
/// Rohwert), beim Commit eine reine Transaktions-Diagnose, bei der Hülle die opake Message-ID und der
/// statische Dispatch-Key. Weder der `partition_key` noch der `content` der Nachricht fließen hinein.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetter {
    message_type: String,
    attempts: u32,
    reason: String,
}

impl DeadLetter {
    /// Liest eine ausgesonderte Nachricht aus den drei Spalten `message_type, attempt,
    /// dead_letter_reason` — die eine Abbildung für die Abfrage **und** das `RETURNING` des
    /// Aussonderns.
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            message_type: row.get(0)?,
            attempts: row.get(1)?,
            reason: row.get(2)?,
        })
    }

    /// Der Dispatch-Key der ausgesonderten Nachricht.
    #[must_use]
    pub fn message_type(&self) -> &str {
        &self.message_type
    }

    /// Wie viele Zustellversuche sie bekam.
    #[must_use]
    pub const fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Der redigierte Grund.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for DeadLetter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} — {} (nach {} Versuch(en))",
            self.message_type, self.reason, self.attempts
        )
    }
}

impl SqliteOutbox {
    /// Verbucht den Ausgang **eines** Versuchs an der Outbox-Zeile. Ein Erfolg ist schon geackt (in der
    /// Transaktion der Unit); ein Fehlschlag wird nach [`disposition`] gezählt oder ausgesondert.
    ///
    /// Läuft **nach** dem Versuch, also nachdem die Unit des Versuchs committet oder fallengelassen
    /// wurde: die Serialitäts-Invariante der Verbindung bleibt gewahrt. Getroffen wird nur eine noch
    /// offene Zeile (`AND status = 'open'`), und was gemeldet wird, kommt aus genau dieser Zeile.
    pub(super) fn settle(
        &self,
        message: &OutboxMessage,
        outcome: Result<(), OutboxError>,
    ) -> Delivery {
        let Err(error) = outcome else {
            return Delivery::Processed;
        };
        let booked = match disposition(&error, message.attempt) {
            Disposition::Redeliver => self
                .with_connection(|conn| {
                    conn.query_row(COUNT_ATTEMPT, params![message.id], |row| row.get(0))
                        .optional()
                })
                .map(|attempts| {
                    attempts.map(|attempts| Delivery::Redelivery {
                        error: error.clone(),
                        attempts,
                    })
                }),
            Disposition::DeadLetter => self
                .with_connection(|conn| {
                    conn.query_row(
                        DEAD_LETTER,
                        params![message.id, error.to_string()],
                        DeadLetter::from_row,
                    )
                    .optional()
                })
                .map(|dead_letter| {
                    dead_letter.map(|dead_letter| Delivery::DeadLettered {
                        error: error.clone(),
                        dead_letter,
                    })
                }),
        };
        match booked {
            Ok(Some(delivery)) => delivery,
            Ok(None) => Delivery::SettledElsewhere(error),
            Err(diagnosis) => Delivery::Unrecorded {
                error,
                diagnosis: diagnosis.to_string(),
            },
        }
    }

    /// Die **ausgesonderten** Nachrichten der Datei in Einreih-Reihenfolge — samt Grund. Damit kann
    /// ein Lauf sie am Ende benennen, statt dass sie still verloren gehen.
    ///
    /// # Errors
    ///
    /// Ein `rusqlite`-Fehler, wenn sich die Outbox nicht lesen lässt.
    pub fn dead_letters(&self) -> rusqlite::Result<Vec<DeadLetter>> {
        self.with_connection(|conn| {
            let mut statement = conn.prepare(SELECT_DEAD_LETTERS)?;
            let rows = statement.query_map([], DeadLetter::from_row)?;
            rows.collect()
        })
    }
}

/// Zählt einen gescheiterten Versuch einer weiter offenen Nachricht und gibt den neuen Stand zurück —
/// keine Zeile, wenn sie nicht mehr offen war.
const COUNT_ATTEMPT: &str = "\
UPDATE outbox
SET attempt    = attempt + 1,
    updated_at = CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE id = ?1 AND status = 'open'
RETURNING attempt;";

/// Zählt den letzten Versuch, nimmt die Nachricht mit ihrem redigierten Grund aus der Zustellung und
/// gibt sie so zurück, wie sie nun steht — keine Zeile, wenn sie nicht mehr offen war.
const DEAD_LETTER: &str = "\
UPDATE outbox
SET status             = 'dead_letter',
    attempt            = attempt + 1,
    dead_letter_reason = ?2,
    updated_at         = CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE id = ?1 AND status = 'open'
RETURNING message_type, attempt, dead_letter_reason;";

/// Nur die drei Spalten, die eine ausgesonderte Nachricht benennen — nie `partition_key` oder `content`
/// (`.rules/sql/queries-und-performance.md`).
const SELECT_DEAD_LETTERS: &str = "\
SELECT message_type, attempt, dead_letter_reason
FROM outbox
WHERE status = 'dead_letter'
ORDER BY rowid;";

#[cfg(test)]
mod tests;
