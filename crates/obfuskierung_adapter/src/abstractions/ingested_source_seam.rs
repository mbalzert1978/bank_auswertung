use crate::models::IngestedRow;

/// Der öffentliche **Lese-Seam** auf den einlesen-Speicher (Roh-Tabelle): liefert die rohe Zeile zu
/// einer Aggregat-ID. **io erfüllt ihn** (liest die einlesen-Roh-Tabelle) — roh-typisiert, ohne
/// Domänen-Kenntnis. Die `SourceBridge` mappt den von der Domäne
/// verlangten Port `obfuskierung_domain::RawBookingSource` darauf (Zeile → Domänen-Aggregat) und
/// deutet ein `None` in den terminalen `SourceError::NotFound`.
pub trait IngestedSourceSeam {
    /// Liest die rohe einlesen-Zeile zur ID (`None` = nicht vorhanden).
    fn read(&self, id: &str) -> Option<IngestedRow>;
}
