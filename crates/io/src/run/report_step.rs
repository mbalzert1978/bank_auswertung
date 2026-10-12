use std::io::Write;

use auswerten_adapter::{HistoryEvaluationRow, OwnAccountRow};

use super::{RunError, RunPaths, RunReport, write_line};
use crate::sqlite::SqliteOutbox;

/// Schreibt den Bericht des gelungenen Laufs — gescopt über **seine** Lauf-Identität (ADR-0016).
pub(super) fn write(
    outbox: &SqliteOutbox,
    paths: &RunPaths,
    protocol: &mut dyn Write,
    report: &RunReport,
    history: HistoryEvaluationRow,
    own_accounts: Vec<OwnAccountRow>,
) -> Result<(), RunError> {
    let facts = crate::report::RunFacts {
        run_id: report.run_id(),
        incomplete: report.incomplete(),
        history,
        own_accounts,
    };
    crate::report::write(outbox, &paths.report, facts)
        .map_err(|crate::report::ReportError(diagnosis)| RunError::Report(diagnosis))?;
    write_line(protocol, "Lauf: Bericht geschrieben")
}
