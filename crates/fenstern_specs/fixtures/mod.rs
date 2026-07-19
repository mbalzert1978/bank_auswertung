//! Die **spec-lokalen** Fixtures/Helfer des fenstern-Slice — Input-Builder, Expected-Builder und
//! Test-Consts, die die nackten Tests in `tests/` per Modul-Include (`#[path = …] mod fixtures;`)
//! hereinziehen. Erwartungsdaten sind test-seitig und gehören darum hierher, nicht in die
//! Adapter-`TestApi`.

use fenstern_adapter::{AnalysisWindowRow, ObfuscatedRow, WindowingTestApi};

/// Die reale Seitengröße des Pagers (ADR-0013, Entscheidung B: bewusst > 365, damit ein voller
/// Jahreszyklus in **eine** Seite fällt). Kommt als **Wert** herein — die Domäne liest keine Config.
pub(crate) const WINDOW_DAYS: u32 = 400;

/// Der Anker eines Laufs: der jüngste Buchungstag als sortierbarer Tag-Stamp (ADR-0013,
/// Entscheidung C). Ein schlichter, gut merkbarer Wert — die Spanne rechnet sich daraus.
pub(crate) const ANKER: i64 = 1000;

/// Die Untergrenze der Seite zum [`ANKER`] bei [`WINDOW_DAYS`] — `anker − window_days + 1`.
pub(crate) const UNTERGRENZE: i64 = ANKER - WINDOW_DAYS as i64 + 1;

/// Eine TestApi mit realer Seitengröße und sonst leerem obfuskiert-Bestand.
pub(crate) fn api() -> WindowingTestApi {
    WindowingTestApi::new().with_window_days(WINDOW_DAYS)
}

/// Eine vollständige, gültige obfuskierte Zeile, wie sie der Lese-Seam aus der `obfuskiert`-Ablage
/// liefert — nur ID (Fremdschlüssel) und Währung; alles Weitere braucht das Fenster nicht.
pub(crate) fn zeile(id: &str, currency: &str) -> ObfuscatedRow {
    ObfuscatedRow {
        id: id.to_string(),
        currency: currency.to_string(),
    }
}

/// Das erwartete abgelegte Auswertungsfenster: die Lauf-Spanne, die eine Währung und die
/// Fremdschlüssel auf genau die eingefrorenen Zeilen.
pub(crate) fn erwartetes_fenster(booking_ids: &[&str]) -> AnalysisWindowRow {
    AnalysisWindowRow {
        from: UNTERGRENZE,
        to: ANKER,
        currency: "EUR".to_string(),
        booking_ids: booking_ids.iter().map(ToString::to_string).collect(),
    }
}
