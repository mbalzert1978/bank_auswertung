use crate::models::Line;

/// Der öffentliche, mechanische **Speicher-Seam**: legt eine bereits auf die primitive Wire-Form
/// abgeflachte [`Line`] ab und **vergibt dabei die Identität** der Zeile — er gibt die zugeteilte
/// ID (Textform) zurück (io: `INSERT … RETURNING id`). **Kein Domänentyp** — io (rusqlite) erfüllt ihn
/// ohne Domänen-Kenntnis; Format/Transaktion liegen bei der Implementierung. Er ist **fehlbar** und
/// **terminal**: ein misslungener Zugriff ist ein [`StoreUnavailable`]. Die
/// `StoreBridge` mappt ihn auf den internen Domänen-Port
/// `einlesen_domain::Store`, indem sie das Aggregat auf die `Line` abflacht (ACL, „innen → außen")
/// und die zurückgegebene ID in die `IngestionId` bzw. den Fehler in den Domänen-Fehler hüllt.
pub trait StoreSeam {
    /// Legt die abgeflachte Zeile ab und gibt die dabei vergebene ID zurück; `Err` = der Zugriff
    /// scheiterte terminal. Der Commit besorgt io.
    fn store(&self, line: &Line) -> Result<String, StoreUnavailable>;
}

/// Die Ablage ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose** trägt: io
/// speist den Text seines eigenen Fehler-Results ein (Pfad, OS-/DB-Fehler), damit
/// nachvollziehbar bleibt, was genau schieflief; der Text bleibt lokal, kein Egress.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Zugriffs-Mechanik** und zitiert **nie** Satz-/Feldinhalt; io muss das beim Erfüllen einhalten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUnavailable {
    /// Die durchgereichte io-Diagnose (Zugriffs-Mechanik, nie Satz-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for StoreUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die
        // Domänen-`Display` von `SinkError::StoreFailed` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StoreUnavailable {}
