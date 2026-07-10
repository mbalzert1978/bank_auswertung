use crate::dto::Line;

/// Der öffentliche, mechanische **Speicher-Seam**: persistiert (staged) eine bereits auf die
/// primitive Wire-Form abgeflachte [`Line`] und **vergibt dabei die Identität** der Zeile — er gibt
/// die zugeteilte ID (Textform) zurück (io: `INSERT … RETURNING id`,
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)).
/// **Kein Domänentyp** — io (sqlx) erfüllt ihn ohne Domänen-Kenntnis; Format/Transaktion liegen bei
/// der Implementierung. Die [`StoreBridge`](crate::bridges::StoreBridge) mappt den internen
/// Domänen-Port `einlesen_domain::Store` (der ein `LineAggregate` staged und eine `IngestionId`
/// zurückgibt) auf ihn, indem sie das Aggregat auf die `Line` abflacht (ACL, „innen → außen:
/// Domänen-VO → DTO") und die zurückgegebene ID in die `IngestionId` hüllt. Vorbild `ISftpPuller`
/// der C#-Referenz: ein roh-typisierter Seam, den nur io erfüllt.
pub trait StoreSeam {
    /// Reiht die abgeflachte Zeile in die offene Arbeitseinheit ein (Staging) und gibt die dabei
    /// vergebene ID zurück; der Commit besorgt io.
    fn store(&self, line: &Line) -> String;
}
