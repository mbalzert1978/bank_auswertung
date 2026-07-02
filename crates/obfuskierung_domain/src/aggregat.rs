use std::collections::HashMap;

use crate::token::{KontoIdEntity, ObfuskierterTextEntity, PseudonymEntity};

/// Das Aggregat des Obfuskierungs-Slice: die First-Seen-Vergabe der Ersatz-Token
/// für eigene Konten (`KONTO_x`) und Personen (`PERSON_x`). Rich domain — der
/// einzige Weg hinein sind die `obfuskiere_*`-Methoden; die Vergabe-Register sind
/// gekapselt.
///
/// **Laufstabil**: derselbe Klartext-Schlüssel erhält im selben Lauf durchgängig
/// dasselbe Token. **Laufübergreifend** stabil wird die Vergabe, sobald sie aus
/// einem geladenen [`Zuordnungsbestand`] gestartet und danach zurückgesichert wird
/// ([ADR-0006](../../../docs/adr/0006-pseudonym-stabilitaet.md)) — die Persistenz
/// dahinter ist vertagt.
#[derive(Debug, Clone, Default)]
pub struct ObfuskierungsAggregat {
    konten: FirstSeen,
    personen: FirstSeen,
}

impl ObfuskierungsAggregat {
    /// Startet die Vergabe aus einem bereits persistierten Bestand — so behalten
    /// die Token ihre laufübergreifende Bedeutung. Ein leerer Bestand ergibt eine
    /// frische Vergabe.
    pub fn aus_bestand(bestand: Zuordnungsbestand) -> Self {
        Self {
            konten: FirstSeen::aus_paaren(bestand.konten),
            personen: FirstSeen::aus_paaren(bestand.personen),
        }
    }

    /// Löst einen (bereits normalisierten) eigenen Konto-Schlüssel zur
    /// obfuskierten `KONTO_x` auf — First-Seen. Ob die IBAN gültig und eine
    /// **eigene** ist, entscheiden vorgelagerte Slices; hier wird nur vergeben.
    pub fn obfuskiere_konto(&mut self, schlüssel: &str) -> KontoIdEntity {
        konto_label(self.konten.index(schlüssel))
    }

    /// Vergibt für einen Empfänger-Namen ein stabiles `PERSON_x` (First-Seen).
    /// `None`, wenn der Name leer ist — dann gibt es niemanden zu
    /// pseudonymisieren.
    pub fn obfuskiere_person(&mut self, name: &str) -> Option<PseudonymEntity> {
        let key = name.trim();
        (!key.is_empty()).then(|| person_label(self.personen.index(key)))
    }

    /// Führt Freitext durch den einen Obfuskierungs-Engpass (aktuell: trimmen +
    /// Leeres ablehnen; Scrubbing vertagt, [ADR-0003](../../../docs/adr/0003-rust-als-sprache.md)).
    /// `None`, wenn nach dem Trimmen nichts bleibt. Bewusst `&self`: künftiges
    /// Scrubbing darf die vergebenen Token kennen.
    pub fn obfuskiere_text(&self, roh: &str) -> Option<ObfuskierterTextEntity> {
        ObfuskierterTextEntity::new(roh)
    }

    /// Der aktuelle Vergabe-Bestand zum Zurücksichern — verlustfrei
    /// (round-trip-vollständig: `aus_bestand` ∘ `bestand` ist die Identität).
    pub fn bestand(&self) -> Zuordnungsbestand {
        Zuordnungsbestand {
            konten: self.konten.paare(),
            personen: self.personen.paare(),
        }
    }
}

/// Der persistierbare Vergabe-Bestand (Klartext-Schlüssel → laufende Nummer),
/// getrennt für Konten und Personen. Undurchsichtiges Round-trip-Token zwischen
/// Aggregat und [`ZuordnungsSpeicher`](crate::ZuordnungsSpeicher): der Store hält
/// und reicht ihn, inspiziert ihn aber nicht. **Ohne serde** — die enthaltenen
/// Klartext-Schlüssel sind strukturelle Identifikatoren und dürfen kein
/// `Serialize` ableiten.
#[derive(Debug, Clone, Default)]
pub struct Zuordnungsbestand {
    konten: Vec<(String, usize)>,
    personen: Vec<(String, usize)>,
}

/// Eine First-Seen-Nummernvergabe: der erste unbekannte Schlüssel bekommt 1, der
/// nächste 2 … und einmal vergeben bleibt fest.
#[derive(Debug, Clone, Default)]
struct FirstSeen {
    map: HashMap<String, usize>,
}

impl FirstSeen {
    fn index(&mut self, key: &str) -> usize {
        let next = self.map.len() + 1;
        *self.map.entry(key.to_string()).or_insert(next)
    }

    /// Die Vergabe als geordnete Paare (nach Nummer aufsteigend) — deterministisch
    /// und lückenlos, sodass `aus_paaren` die Zählung korrekt fortsetzt.
    fn paare(&self) -> Vec<(String, usize)> {
        let mut paare: Vec<(String, usize)> = self
            .map
            .iter()
            .map(|(key, number)| (key.clone(), *number))
            .collect();
        paare.sort_by_key(|(_, number)| *number);
        paare
    }

    fn aus_paaren(paare: Vec<(String, usize)>) -> Self {
        Self {
            map: paare.into_iter().collect(),
        }
    }
}

/// Baut ein `KONTO_x`-Label. Konstruktionsbedingt nie leer (Präfix + Zahl ≥ 1),
/// daher der `expect` — er kann nicht feuern.
fn konto_label(n: usize) -> KontoIdEntity {
    KontoIdEntity::new(format!("KONTO_{n}"))
        .expect("KONTO-Label ist konstruktionsbedingt nicht leer")
}

/// Baut ein `PERSON_x`-Label. Konstruktionsbedingt nie leer, daher der `expect`.
fn person_label(n: usize) -> PseudonymEntity {
    PseudonymEntity::new(format!("PERSON_{n}"))
        .expect("PERSON-Label ist konstruktionsbedingt nicht leer")
}
