//! **Was nach einem Zustellversuch mit der Nachricht geschieht**
//! ([Issue 0085](../../../../docs/issues/0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md)):
//! die Versuchszählung und das Aussondern (Dead-Letter) des Brokers. Das beantwortet die in
//! [ADR-0012](../../../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
//! vertagte Frage, **wie `io` einen terminalen Fehlschlag surfacet**: es nimmt die Nachricht aus der
//! Zustellung, legt den redigierten Grund an ihre Outbox-Zeile und lässt den Lauf sie benennen.
//!
//! **Form: eine Status-Spalte in der Outbox, keine eigene Tabelle.** Eine ausgesonderte Nachricht
//! bleibt dieselbe Zeile mit `status = 'dead_letter'`; sie wandert nirgendwohin, und zurückholen hieße
//! nur, den Status wieder auf `open` zu setzen.
//!
//! **Der Slice merkt davon nichts.** Er gibt weiterhin nur seinen nackten, redigierten Domänen-Fehler
//! zurück (ADR-0012); Retry und Dead-Letter sind Broker-Mechanik und leben allein hier.

use rusqlite::params;

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

/// Der Ausgang **eines** Zustellversuchs, nachdem der Broker ihn verbucht hat — das, was
/// [`deliver_open`](SqliteOutbox::deliver_open) je Nachricht zurückgibt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// Verarbeitet und geackt.
    Processed,
    /// **Transient** gescheitert (`Retry`) und noch Versuche übrig: die Nachricht bleibt offen, der
    /// Versuch ist gezählt, die nächste Runde stellt sie erneut zu.
    Redelivery(OutboxError),
    /// Aus der Zustellung genommen — sofort bei einem **terminalen** Fehlschlag (`Warning`/`Critical`),
    /// bei einem transienten nach [`MAX_DELIVERY_ATTEMPTS`] Versuchen. Der Grund liegt redigiert an
    /// der Outbox-Zeile ([`SqliteOutbox::dead_letters`]).
    DeadLettered(OutboxError),
}

impl Delivery {
    /// Wurde die Nachricht verarbeitet?
    #[must_use]
    pub const fn is_processed(&self) -> bool {
        matches!(self, Self::Processed)
    }
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
    /// Die ausgesonderte Nachricht, wie der Broker sie aus **diesem** Versuch verbucht — die eine
    /// Quelle für den abgelegten Grund und für das, was der Lauf darüber meldet.
    pub(crate) fn of(message: &OutboxMessage, error: &OutboxError) -> Self {
        Self {
            message_type: message.message_type.clone(),
            attempts: message.attempt.saturating_add(1),
            reason: error.to_string(),
        }
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
    /// Transaktion der Unit); ein Fehlschlag wird hier gezählt und — je nach Art und Versuchszahl —
    /// ausgesondert.
    ///
    /// Läuft **nach** dem Versuch, also nachdem die Unit des Versuchs committet oder fallengelassen
    /// wurde: die Serialitäts-Invariante der Verbindung bleibt gewahrt. Getroffen wird nur eine noch
    /// offene Zeile (`AND status = 'open'`); war die Nachricht schon anderweitig verarbeitet, ändert
    /// sich nichts.
    pub(super) fn settle(
        &self,
        message: &OutboxMessage,
        outcome: Result<(), OutboxError>,
    ) -> rusqlite::Result<Delivery> {
        let Err(error) = outcome else {
            return Ok(Delivery::Processed);
        };
        let leaves_delivery = match error {
            OutboxError::Retry { .. } => message.attempt.saturating_add(1) >= MAX_DELIVERY_ATTEMPTS,
            OutboxError::Warning { .. } | OutboxError::Critical { .. } => true,
        };
        if leaves_delivery {
            let dead_letter = DeadLetter::of(message, &error);
            self.with_connection(|conn| {
                conn.execute(DEAD_LETTER, params![message.id, dead_letter.reason])
            })?;
            Ok(Delivery::DeadLettered(error))
        } else {
            self.with_connection(|conn| conn.execute(COUNT_ATTEMPT, params![message.id]))?;
            Ok(Delivery::Redelivery(error))
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
            let rows = statement.query_map([], |row| {
                Ok(DeadLetter {
                    message_type: row.get(0)?,
                    attempts: row.get(1)?,
                    reason: row.get(2)?,
                })
            })?;
            rows.collect()
        })
    }
}

/// Zählt einen gescheiterten Versuch einer weiter offenen Nachricht.
const COUNT_ATTEMPT: &str = "\
UPDATE outbox
SET attempt    = attempt + 1,
    updated_at = CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE id = ?1 AND status = 'open';";

/// Zählt den letzten Versuch und nimmt die Nachricht mit ihrem redigierten Grund aus der Zustellung.
const DEAD_LETTER: &str = "\
UPDATE outbox
SET status             = 'dead_letter',
    attempt            = attempt + 1,
    dead_letter_reason = ?2,
    updated_at         = CAST(unixepoch('subsec') * 1000 AS INTEGER)
WHERE id = ?1 AND status = 'open';";

/// Nur die drei Spalten, die eine ausgesonderte Nachricht benennen — nie `partition_key` oder `content`
/// (`.rules/sql/queries-und-performance.md`).
const SELECT_DEAD_LETTERS: &str = "\
SELECT message_type, attempt, dead_letter_reason
FROM outbox
WHERE status = 'dead_letter'
ORDER BY rowid;";

#[cfg(test)]
mod tests;
