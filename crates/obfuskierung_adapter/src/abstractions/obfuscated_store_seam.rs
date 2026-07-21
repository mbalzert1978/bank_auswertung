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

/// Die Ablage ist **misslungen** — terminaler Seam-Fehler, roh-typisiert und ohne Domänen-Kenntnis.
/// Die io-seitige Diagnose (Transaktions-/DB-Mechanik) bleibt lokal am Seam; über die Adapter-Grenze
/// reicht die `RepositoryBridge` nur den redigierten Domänen-Fehler
/// (opake ID, kein Rohwert). Vorbild: `SourceUnavailable` des Einlese-Slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUnavailable;
