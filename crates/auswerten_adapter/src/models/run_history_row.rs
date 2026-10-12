use auswerten_domain::{NonEmptyString, RunHistory, SeedMarker};

use crate::models::WindowBookingRow;
use crate::models::analysis_window_row::MissingMandatoryField;

/// Die **Historie eines Laufs**, wie der fenster-übergreifende Lese-Seam
/// ([`OccurrenceSeam`](crate::OccurrenceSeam)) sie für eine Lauf-Spanne liefert: die Buchungen
/// **aller** Fenster des Laufs und das **opake Kennzeichen der Tokenisierung**, unter der ihre Token
/// entstanden — **nicht** der Seed (ADR-0010).
///
/// Reines Zwischen-DTO zwischen dem Seam (io) und der Domäne; die Abbildung „außen → innen" lebt
/// hier (ACL). **Kein serde**: io baut die Zeile aus dem Speicher.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunHistoryRow {
    /// Der Seed-Marker der Buchungen — **leer**, wenn in der Spanne keine Buchung liegt (dann gibt es
    /// keine Tokenisierung, die er benennen könnte).
    pub seed_marker: String,
    pub bookings: Vec<WindowBookingRow>,
}

impl RunHistoryRow {
    /// Deutet die rohe Zeile in die Domänen-[`RunHistory`] (ACL, „außen → innen").
    ///
    /// **Ohne Buchung ist der Marker kein Pflichtfeld** — eine leere Spanne ist eine gültige Auskunft.
    /// **Mit** Buchungen ist er es: fehlt er dann, ließe sich nicht prüfen, ob die Gegenkonto-Token
    /// zur Eigene-Konten-Config passen, und das Netting liefe blind. Das endet terminal mit dem
    /// statischen Feldnamen, nie mit einem Rohwert.
    pub(crate) fn to_history(&self) -> Result<RunHistory, MissingMandatoryField> {
        if self.bookings.is_empty() {
            return Ok(RunHistory::empty());
        }
        let seed_marker = SeedMarker::from(
            NonEmptyString::new(&self.seed_marker)
                .ok_or(MissingMandatoryField::named("seed_marker"))?,
        );
        Ok(RunHistory::rehydrate(
            seed_marker,
            self.bookings
                .iter()
                .map(WindowBookingRow::to_booking)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
