/// Der **Seed** der Obfuskierung: der lokal bleibende Schlüssel, mit dem der
/// [`Obfuscator`](crate::Obfuscator)-Port aus einem Klartext deterministisch ein Ersatz-Token
/// erzeugt. Gleicher Seed + gleiche Eingabe → gleiches Token (laufstabil, solange derselbe Seed
/// injiziert wird); mit Seed **und** Eingang lässt sich das Token von außen reproduzieren.
///
/// Der Seed wird in die obfuskierte Buchung **mitpersistiert** (Reproduzierbarkeit;
/// [ADR-0006](../../../docs/adr/0006-pseudonym-stabilitaet.md)-Nachfolge: Stabilität ruht auf
/// dem Seed, nicht auf einer Klartext↔Token-Mapping-Tabelle). Er verlässt die Maschine nie —
/// **kein serde** (die Domäne ist dependency-frei ohnehin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seed(u64);

impl Seed {
    /// Baut den Seed aus seinem Rohwert (der Adapter reicht ihn aus dem `SeedSeam`).
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Der Rohwert — der Obfuscator-Port keyt seine deterministische Vergabe damit.
    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }
}
