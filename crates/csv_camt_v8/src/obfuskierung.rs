use std::collections::HashMap;

use domain::{KontoId, Pseudonym};
use eigene_konten::{EigeneKonten, Iban, Kontozuordnung};

use crate::fehler::ImportError;

/// Die obfuskierenden Vergabe-Register eines einzelnen Importlaufs: ordnet
/// Klartext (eigene IBAN bzw. Empfänger-Name) einem stabilen Ersatz-Token zu.
/// First-Seen und **nur laufstabil** — ein Register lebt genau einen Import; die
/// laufübergreifend persistente Zuordnung kommt mit einem späteren Slice.
#[derive(Default)]
pub(crate) struct Obfuskierung {
    konten: FirstSeen,
    personen: FirstSeen,
}

impl Obfuskierung {
    /// Löst ein Auftragskonto (eigene IBAN) zur obfuskierten `KONTO_x` auf. Das
    /// Konto muss ein in der Eigene-Konten-Config hinterlegtes eigenes sein
    /// (`über die Config`); andernfalls bricht der Import laut ab. Der
    /// First-Seen-Schlüssel ist die **normalisierte** IBAN, sodass verschiedene
    /// Schreibweisen desselben Kontos dasselbe `KONTO_x` erhalten.
    pub(crate) fn konto_id(
        &mut self,
        own_iban: &str,
        accounts: &EigeneKonten,
    ) -> Result<KontoId, ImportError> {
        let iban = Iban::parse(own_iban).map_err(ImportError::AuftragskontoInvalid)?;
        match accounts.lookup(&iban) {
            Kontozuordnung::Eigen(_) => Ok(konto_label(self.konten.index(iban.as_str()))),
            Kontozuordnung::Fremd => Err(ImportError::AuftragskontoUnknown),
        }
    }

    /// Vergibt für einen Empfänger-Namen ein stabiles `PERSON_x` (First-Seen):
    /// derselbe Klartextname erhält im Lauf durchgängig dasselbe Pseudonym.
    /// `None`, wenn der Name leer ist — dann gibt es niemanden zu
    /// pseudonymisieren.
    pub(crate) fn pseudonym(&mut self, name: &str) -> Option<Pseudonym> {
        let key = name.trim();
        (!key.is_empty()).then(|| person_label(self.personen.index(key)))
    }
}

/// Eine First-Seen-Nummernvergabe: der erste unbekannte Schlüssel bekommt 1, der
/// nächste 2 … und einmal vergeben bleibt fest.
#[derive(Default)]
struct FirstSeen {
    map: HashMap<String, usize>,
}

impl FirstSeen {
    fn index(&mut self, key: &str) -> usize {
        let next = self.map.len() + 1;
        *self.map.entry(key.to_string()).or_insert(next)
    }
}

/// Baut ein `KONTO_x`-Label. Konstruktionsbedingt nie leer (Präfix + Zahl ≥ 1),
/// daher der `expect` — er kann nicht feuern.
fn konto_label(n: usize) -> KontoId {
    KontoId::new(format!("KONTO_{n}")).expect("KONTO-Label ist konstruktionsbedingt nicht leer")
}

/// Baut ein `PERSON_x`-Label. Konstruktionsbedingt nie leer, daher der `expect`.
fn person_label(n: usize) -> Pseudonym {
    Pseudonym::new(format!("PERSON_{n}")).expect("PERSON-Label ist konstruktionsbedingt nicht leer")
}
