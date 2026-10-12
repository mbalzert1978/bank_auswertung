//! **Der HTML-Bericht** ([Issue 0012](../../../../docs/issues/0012-html-bericht.md)): die Ausgabe
//! dieses Programms — die **Analyse der Domäne**, nicht „was das LLM sagte" (ADR-0004).
//!
//! **Wann er entsteht** ([ADR-0016](../../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)):
//! am Ende eines Laufs, der die Outbox leer laufen lässt (Exit 0), und er liest **genau** die
//! `run_id`, die dieser Lauf gemintet hat. Nach Exit 1 gibt es keinen Bericht; eine ältere Datei am
//! Ziel bleibt dann unberührt. Geschrieben wird in eine Nachbar-Datei, die erst am Ende auf das Ziel
//! umbenannt wird — ein halber Bericht liegt nie am Ziel.
//!
//! **Woraus er besteht:**
//!
//! - das Ergebnis der fenster-übergreifenden Reichweite **dieses** Laufs (Cashflow je Konto und
//!   Monat, Pareto-Ranking, Spar-Kandidaten, Wiederkehr), wie der Lauf es entgegengenommen hat;
//! - die je Fenster abgelegten Umbuchungen und Gebühren des Laufs ([`source`]);
//! - die Kategorie je Treiber **allein** aus dem Kategorie-Register
//!   (`SqliteCategoryRegistry::categories_of`); fehlt ein Eintrag, steht „unbekannt" da;
//! - wie viele Buchungen der Analyse fehlen (ADR-0017) — dann steht oben ein Hinweis.
//!
//! **Warum hier und keine eigene Slice-Triade:** der Bericht rechnet nichts — er beschriftet und
//! formatiert, was die Domäne errechnet hat. HTML ist Format-Wissen und gehört darum nach `io`, wie
//! das CSV-Format beim Einlesen. Eine Triade (`report_domain`/`_adapter`/`_specs`) hätte keine
//! Domänen-Regel zu tragen; kein ADR verlangt sie (YAGNI).

use std::path::Path;

use auswerten_adapter::{HistoryEvaluationRow, OwnAccountRow};

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
    /// Die tokenisierte Eigene-Konten-Config — die Quelle der Konto-Rollen.
    pub(crate) own_accounts: Vec<OwnAccountRow>,
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
    let details = outbox
        .with_connection(|conn| source::read_run(conn, facts.run_id))
        .map_err(|error| ReportError(error.to_string()))?;
    let categories = SqliteCategoryRegistry::new(outbox.clone())
        .categories_of(&model::driver_fingerprints(&facts.history))
        .map_err(|error| ReportError(error.message))?;
    let bericht = model::Bericht::compose(&model::ReportInputs {
        incomplete: facts.incomplete,
        history: facts.history,
        transfers: details.transfers,
        fees: details.fees,
        categories,
        own_accounts: facts.own_accounts,
        company_names: details.company_names,
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
