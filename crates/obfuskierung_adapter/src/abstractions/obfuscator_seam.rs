/// Der öffentliche, mechanische **Obfuskierungs-Seam**: erzeugt aus (Seed, Klartext)
/// deterministisch einen Ersatzwert-Text. **io erfüllt ihn** (eine Random-/Hash-Bibliothek hinter
/// dem Interface) — roh-typisiert (`u64`/`&str`/`String`), ohne Domänen-Kenntnis. Die
/// [`ObfuscatorBridge`](crate::bridges::ObfuscatorBridge) mappt den von der Domäne verlangten Port
/// `obfuskierung_domain::Obfuscator` auf ihn. Vorbild: `MoneySeam`/`CalendarSeam` des Einlese-Slice.
pub trait ObfuscatorSeam {
    /// Der deterministische Ersatzwert für einen Klartext unter dem Seed (nicht-leer zugesichert).
    fn obfuscate(&self, seed: u64, plaintext: &str) -> String;
}
