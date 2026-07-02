use crate::aggregat::Zuordnungsbestand;

/// Fehler des [`ZuordnungsSpeicher`]-Ports — bewusst offen (Datei-I/O, DB, …).
/// Nur über `std` typisiert (die Domäne bindet keine Fehler-Bibliothek); der
/// Adapter mappt/umschließt ihn.
pub type SpeicherError = Box<dyn std::error::Error + Send + Sync>;

/// Der **von der Domäne verlangte** Port zum persistenten Klartext↔Token-Store
/// ([ADR-0006](../../../docs/adr/0006-pseudonym-stabilitaet.md)): lädt den bisher
/// vergebenen [`Zuordnungsbestand`] (damit `KONTO_x`/`PERSON_x` laufübergreifend
/// stabil bleiben) und sichert den fortgeschriebenen zurück.
///
/// Der Store hält als Einziger die Klartext-Abbildung und verlässt die Maschine
/// nie — getrennt vom obfuskierten `Speicher`-Port. Die **Persistenz selbst ist
/// vertagt** (kommt mit dem `io`-Crate, [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md));
/// aktuell erfüllt ein laufstabiler In-Memory-Fake diesen Port. Für echtes IO wird
/// der Port später über einen Adapter-Seam gebrückt (io hängt nie an `_domain`).
pub trait ZuordnungsSpeicher {
    /// Lädt den bisher vergebenen Bestand (leer bei jungfräulichem Store).
    fn laden(&self) -> Result<Zuordnungsbestand, SpeicherError>;

    /// Sichert den im Lauf fortgeschriebenen Bestand zurück.
    fn sichern(&mut self, bestand: &Zuordnungsbestand) -> Result<(), SpeicherError>;
}
