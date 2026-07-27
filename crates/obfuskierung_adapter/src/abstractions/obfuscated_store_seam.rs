use crate::models::ObfuscatedBooking;

/// Der öffentliche **Speicher-Seam** der `obfuskiert`-Tabelle: legt (staged) eine obfuskierte Buchung
/// ab und gibt die dabei **neu vergebene** ID zurück (io: `INSERT … RETURNING id`). **io erfüllt ihn**
/// (rusqlite) — die Tabelle ist die Wahrheit für stromabwärts; den Commit (Tabelle **und** die
/// `Obfuscated`-Nachricht atomar in **einer** Arbeitseinheit) besorgt io.
///
/// **Fehlbar und terminal:** ein Scheitern ist ein seam-lokaler [`StoreUnavailable`] (roh-typisiert,
/// ohne Domänen-Kenntnis); die `RepositoryBridge` mappt ihn „außen
/// → innen" auf den terminalen Domänen-`SourceError` (ACL).
pub trait ObfuscatedStoreSeam {
    /// Reiht die obfuskierte Buchung ein (Staging) und gibt die neu vergebene ID zurück, oder
    /// scheitert terminal mit [`StoreUnavailable`].
    fn save(&self, booking: &ObfuscatedBooking) -> Result<String, StoreUnavailable>;
}

/// Die Ablage ist **misslungen** — terminaler Seam-Fehler, der die **io-Diagnose** trägt
/// (Zugriffs-Mechanik: DB-/Transaktions-Fehler). Die `RepositoryBridge` reicht sie als `reason` in den
/// redigierten Domänen-Fehler `SourceError::StorageUnavailable` weiter (neben der opaken Quell-ID),
/// damit ein unerreichbarer Store diagnostizierbar bleibt — lokal, **kein Egress**: in ein
/// ausgehendes DTO fließt sie nie.
///
/// **Redaktions-Pflicht am Seam (Vertrag, nicht Konstruktion):** die Meldung beschreibt die
/// **Zugriffs-Mechanik** und zitiert **nie** Zeilen-/Feldinhalt; io muss das beim Erfüllen einhalten —
/// weil sie durchgereicht wird, ist genau das die Bedingung dafür, dass der Domänen-Fehler redigiert
/// bleibt.
/// Seit [ADR-0015](../../../../docs/adr/0015-generischer-slice-store-ueber-storespec.md) (Punkt 5)
/// **formgleich** mit den `StoreUnavailable` der drei anderen Slices — die frühere feldlose
/// Sonderform war eine Drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUnavailable {
    /// Die durchgereichte io-Diagnose (Zugriffs-Mechanik, nie Zeilen-/Feldinhalt).
    pub message: String,
}

impl std::fmt::Display for StoreUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nur die Diagnose selbst — den terminalen Meldungssatz besitzt allein die Domänen-`Display`
        // von `SourceError::StorageUnavailable` (kein Satz-Duplikat über Crates).
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StoreUnavailable {}
