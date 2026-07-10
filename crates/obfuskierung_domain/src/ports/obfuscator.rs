use crate::value_objects::{NonEmptyString, Replacement, Seed};

/// Der von der Domäne **verlangte** Port, der aus (Seed, Klartext) deterministisch einen
/// [`Replacement`]-Ersatzwert erzeugt — die eigentliche Obfuskierungs-Mechanik. **io erfüllt
/// ihn** (eine Random-/Hash-Bibliothek hinter dem Interface,
/// [ADR-0003](../../../docs/adr/0003-rust-als-sprache.md)); die Domäne kennt das Verfahren nicht,
/// nur seinen Vertrag: **gleicher Seed + gleiche Eingabe → gleicher Ersatzwert** (deterministisch,
/// laufstabil, von außen mit Seed + Eingang reproduzierbar). Vorbild: die `Calendar`/`Money`-Ports
/// des Einlese-Slice (Format-Mechanik hinter dem Seam, io-erfüllt).
///
/// Der Klartext reist als Domänen-Wert-Objekt [`NonEmptyString`] herein — die Domäne spricht am
/// Port ihre eigenen Typen, nicht primitives `&str` (ADR-0005). Die Bridge zum io-Seam mappt.
pub trait Obfuscator {
    /// Erzeugt den deterministischen Ersatzwert für einen Klartext unter dem gegebenen Seed. Der
    /// Ersatzwert trägt den rohen Klartext nicht — er ersetzt ihn.
    fn obfuscate(&self, seed: Seed, plaintext: &NonEmptyString) -> Replacement;
}
