use crate::abstractions::SourceUnavailable;
use crate::models::WindowBookingRow;

/// Der öffentliche **Lese-Seam** der fenster-übergreifenden Reichweite: liefert die Buchungen
/// **aller** Auswertungsfenster **eines** Laufs — gescopt über die **Lauf-Spanne** `[from … to]`
/// (beide Grenzen einschließlich), die `fenstern` im `HistorieVollständig`-Signal mitträgt
/// (ADR-0013, Entscheidung E). **io erfüllt ihn** — roh-typisiert, ohne Domänen-Kenntnis; in den
/// Specs trägt ihn ein Fake. Die
/// [`OccurrenceBridge`](crate::adapter::OccurrenceBridge) mappt den von der Domäne verlangten Port
/// `auswerten_domain::OccurrenceSource` darauf.
///
/// **Das Scoping lebt hier, nicht im Handler:** dass io die Spanne als `WHERE`-Klausel über die
/// Fenster-Tabelle legt, ist Beschaffungs-Mechanik. Die Domäne verlangt „die Buchungen **dieses**
/// Laufs" und reicht dafür die Spanne herein — sie blättert nicht selbst durch Fenster.
///
/// **Eine Query über viele Fenster, bewusst:** io setzt das Ergebnis aus vielen Fenster-Zeilen und
/// ihrem Join auf die `obfuskiert`-Ablage zusammen — dieselbe Linie, die der
/// [`AnalysisWindowSeam`](crate::AnalysisWindowSeam) beim Join zieht. Ein Seam „gib mir erst die
/// Fenster-IDs" trüge die Zerlegung der Ablage in die Domäne und machte N+1 Lesevorgänge daraus.
///
/// **Reines Read, ohne Gegenstück:** es gibt keinen Schreib-Seam dazu. Die fenster-übergreifende
/// Sicht wird **nicht** persistiert (kein Vorkommens-/Fold-Log) — sie entsteht bei jeder Query neu.
///
/// **Fehlbar und terminal:** ein `Err` ist der Fehlschlag der Beschaffung ([`SourceUnavailable`],
/// io-Diagnose). Ein **leerer** Vektor ist dagegen kein Fehler, sondern die gültige Auskunft „in
/// dieser Spanne liegt nichts" — anders als beim Lesen **eines** Fensters per ID gibt es hier keinen
/// ins Leere zeigenden Zeiger, den man von „nichts gefunden" unterscheiden müsste.
pub trait OccurrenceSeam {
    /// Liest die Buchungen aller Fenster **innerhalb** der Lauf-Spanne (beide Grenzen
    /// einschließlich); `Err` = die Beschaffung misslang (terminal).
    fn bookings_in(&self, from: i64, to: i64) -> Result<Vec<WindowBookingRow>, SourceUnavailable>;
}
