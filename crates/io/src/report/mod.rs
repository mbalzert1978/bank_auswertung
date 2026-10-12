//! **Der HTML-Bericht** ([Issue 0012](../../../../docs/issues/0012-html-bericht.md)): die Ausgabe
//! dieses Programms — die **Analyse der Domäne**, nicht „was das LLM sagte" (ADR-0004).
//!
//! **Was er ist** ([ADR-0018](../../../../docs/adr/0018-bericht-als-io-formatierer-am-ende-eines-laufs.md)):
//! ein `io`-Formatierer am Ende eines Exit-0-Laufs — kein Outbox-Consumer, kein eigener Port, keine
//! eigene Crate.
//!
//! **Wann er entsteht** ([ADR-0016](../../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
//! am Ende eines Laufs, der die Outbox leer laufen lässt (Exit 0), und er liest **genau** die
//! `run_id`, die dieser Lauf gemintet hat. Nach Exit 1 gibt es keinen Bericht; eine ältere Datei am
//! Ziel bleibt dann unberührt. Geschrieben wird in eine Nachbar-Datei, die erst am Ende auf das Ziel
//! umbenannt wird — ein halber Bericht liegt nie am Ziel.
//!
//! **Woraus er besteht:**
//!
//! - das Ergebnis der fenster-übergreifenden Reichweite **dieses** Laufs, wie der Lauf es
//!   entgegengenommen hat — die **einzige** Quelle jeder Zahl: Cashflow je Konto und Monat,
//!   Umbuchungen, Gebühren, Pareto-Ranking, Spar-Kandidaten, Wiederkehr;
//! - die gescrubbten Firmennamen der Buchungen des Laufs ([`source`]), gefiltert gegen die lokale
//!   Namensliste;
//! - die Kategorie je Treiber **allein** aus dem Kategorie-Register
//!   (`SqliteCategoryRegistry::categories_of`); fehlt ein Eintrag, steht „unbekannt" da;
//! - wie viele Buchungen der Analyse fehlen (ADR-0017) — dann steht oben ein Hinweis.
//!

use std::path::Path;

use auswerten_adapter::HistoryEvaluationRow;

use crate::auswerten::SqliteCategoryRegistry;
use crate::sqlite::SqliteOutbox;

mod file;
mod html;
mod model;
mod source;

#[cfg(test)]
pub(crate) use file::neighbour as neighbour_of;

/// Was der Lauf über sich selbst weiß und dem Bericht übergibt.
pub(crate) struct RunFacts<'a> {
    /// Die Lauf-Identität, die dieser Lauf gemintet hat — der Scope jeder Abfrage.
    pub(crate) run_id: &'a str,
    /// Wie viele Buchungen der Analyse fehlen (`RunReport::incomplete`).
    pub(crate) incomplete: usize,
    /// Das Ergebnis der fenster-übergreifenden Reichweite dieses Laufs — leer, wenn der Bestand leer
    /// war und kein Fenster entstand.
    pub(crate) history: HistoryEvaluationRow,
    /// Die lokale Namensliste (`personen.toml`) — ein Firmenname, der einen davon nennt, erscheint
    /// nicht; dann steht nur die Gläubiger-ID da.
    pub(crate) persons: Vec<String>,
}

/// Warum kein Bericht entstand. Redigiert: eine io-Diagnose (Zugriffs-Mechanik), nie ein Zeileninhalt
/// und nie der Zielpfad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportError(pub(crate) String);

/// Liest, was der Bericht des Laufs braucht, setzt ihn zusammen und schreibt ihn nach `target`.
pub(crate) fn write(
    outbox: &SqliteOutbox,
    target: &Path,
    facts: RunFacts<'_>,
) -> Result<(), ReportError> {
    let company_names = outbox
        .with_connection(|conn| source::company_names(conn, facts.run_id))
        .map_err(|error| ReportError(error.to_string()))?;
    let categories = SqliteCategoryRegistry::new(outbox.clone())
        .categories_of(&model::driver_fingerprints(&facts.history))
        .map_err(|error| ReportError(error.message))?;
    let bericht = model::Bericht::compose(&model::ReportInputs {
        incomplete: facts.incomplete,
        history: facts.history,
        categories,
        company_names,
        persons: facts.persons,
    });
    file::replace(target, &bericht.render()).map_err(|error| ReportError(error.to_string()))
}

#[cfg(test)]
mod datenschutz {
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_not_impl_any;

    use super::model::{
        Bericht, CashflowLine, FeeLine, RankLine, RecurrenceLine, SavingsLine, TransferLine,
    };

    // Der Bericht ist der **Egress** des Programms: eine Datei, die die Maschine verlassen kann. Ihn
    // per `derive` serialisierbar zu machen — und damit eine Abkürzung zu bauen, an der Beschriftung
    // vorbei — ist ein **Compile-Fehler**, keine Review-Beobachtung.
    assert_not_impl_any!(Bericht: Serialize, Deserialize<'static>);
    assert_not_impl_any!(CashflowLine: Serialize, Deserialize<'static>);
    assert_not_impl_any!(TransferLine: Serialize, Deserialize<'static>);
    assert_not_impl_any!(FeeLine: Serialize, Deserialize<'static>);
    assert_not_impl_any!(RankLine: Serialize, Deserialize<'static>);
    assert_not_impl_any!(SavingsLine: Serialize, Deserialize<'static>);
    assert_not_impl_any!(RecurrenceLine: Serialize, Deserialize<'static>);
}

#[cfg(test)]
mod tests;
