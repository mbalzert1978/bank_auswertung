use crate::dto::Line;

/// Der öffentliche, mechanische **Speicher-Seam**: persistiert (staged) eine bereits auf die
/// primitive Wire-Form abgeflachte [`Line`]. **Kein Domänentyp** — io (sqlx) erfüllt ihn ohne
/// Domänen-Kenntnis; Format/Transaktion liegen bei der Implementierung. Die
/// [`StoreBridge`](crate::bridges::StoreBridge) mappt den internen Domänen-Port
/// `einlesen_domain::Store` (der ein `LineAggregate` staged) auf ihn, indem sie das
/// Aggregat auf die `Line` abflacht (ACL, „innen → außen: Domänen-VO → DTO"). Vorbild
/// `ISftpPuller` der C#-Referenz: ein roh-typisierter Seam, den nur io erfüllt.
pub trait StoreSeam {
    /// Reiht die abgeflachte Zeile in die offene Arbeitseinheit ein (Staging); der Commit
    /// besorgt io.
    fn store(&self, line: &Line);
}
