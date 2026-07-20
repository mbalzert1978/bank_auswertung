//! Der **geteilte** Commit-Fehler der io-Integrationstests: `obfuskierung` und `fenstern`
//! arrangieren beide einen scheiternden Commit, um den **einzigen** io-internen Retry-Ursprung zu
//! treiben — derselbe Fake-Fehler, darum eine Datei statt zweier Kopien. `einlesen` braucht ihn
//! nicht (sein Fake-Commit ist `Infallible`) und zieht ihn folglich auch nicht herein.

use std::fmt;

/// Ein Commit ist gescheitert (io-interner Retry-Ursprung). Trägt eine **Transaktions**-Diagnose,
/// nie einen Roh-Zeilenwert (Redaktions-Pflicht des Commit-Pfades).
#[derive(Debug)]
pub(crate) struct CommitFailed;

impl fmt::Display for CommitFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "commit fehlgeschlagen (broker weg)")
    }
}

impl std::error::Error for CommitFailed {}
