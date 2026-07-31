//! Die reale io-Erfüllung des öffentlichen [`ObfuscatorSeam`] des `obfuskierung`-Adapters
//! ([Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md),
//! [ADR-0010](../../../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)): der
//! **deterministische, keyed Hash** hinter dem Seam, der aus `(Seed, Klartext)` einen stabilen
//! Ersatzwert erzeugt — das Gegenstück zu `einlesen`s realen `Calendar`/`Money`-Seams. Er tritt an
//! die Stelle des `pub(crate)`-Fakes der Specs (den `io` gar nicht sieht), sodass der
//! `obfuskierung`-Slice end-to-end **ohne einen einzigen Fake** läuft.
//!
//! **Seed-basierte Stabilität ohne Zuordnungstabelle (ADR-0010):** gleicher Seed + gleicher Klartext
//! → gleicher Ersatzwert, **zustandslos** — kein First-Seen-Zähler, keine Klartext↔Token-Tabelle.
//! Die Domäne formt daraus die sprechenden `KONTO_…`/`PERSON_…`-Token; dieser Seam liefert nur den
//! rohen Ersatzwert (32 Hex-Zeichen = 128 Bit).
//!
//! **Battle-tested Lib, keyed:** die Hash-Mechanik kommt aus [`siphasher`] (SipHash-1-3, 128-Bit),
//! **nicht** hand-gerollt — der Seed keyt den Hash (`new_with_keys`), was genau die von ADR-0010
//! verlangte **keyed** Ableitung ist, und `siphasher` ist über Rust-Versionen **stabil** (anders als
//! `std`s `DefaultHasher`, dessen Ausgabe driften darf und der zudem zufällig gekeyt ist). So bleibt
//! ein Token import-übergreifend reproduzierbar. **Ehrliche Benennung — kein Krypto-Versprechen:**
//! SipHash ist ein schneller keyed **PRF-artiger** Hash, keine kryptografische Einwegfunktion. Der
//! Datenschutz ruht **nicht** auf seiner Unumkehrbarkeit, sondern auf der **Maschinen-Grenze**: der
//! rohe Klartext steckt **nicht** im Ersatzwert, und kein Egress-DTO trägt Seed oder Rohwert
//! (compilergeprüfter „kein `Serialize`"-Guard). Die breite 128-Bit-Ausgabe hält die Token-Kollision
//! (die zwei Konten zu **einem** Pseudonym verschmelzen würde) praktisch aus.

use std::hash::Hasher;

use obfuskierung_adapter::ObfuscatorSeam;
use siphasher::sip128::{Hasher128, SipHasher13};

/// Der reale [`ObfuscatorSeam`]: ein zustandsloser, seed-gekeyter Hash über [`siphasher`]. Hält
/// **keinen** Zustand (der Seed reist je Aufruf als Wert herein) — ein Einheits-Typ, den der
/// Aufrufer per Referenz herreicht.
#[derive(Debug, Default)]
pub struct SeededObfuscator;

impl ObfuscatorSeam for SeededObfuscator {
    /// Der deterministische Ersatzwert (32 Hex-Zeichen, 128 Bit): SipHash-1-3, mit dem Seed als
    /// Schlüssel (`new_with_keys(seed, !seed)`) über die Klartext-Bytes. Nicht-leer per Konstruktion;
    /// der Klartext ist aus dem Hash nicht rekonstruierbar (kein Byte reist mit).
    fn obfuscate(&self, seed: u64, plaintext: &str) -> String {
        let mut hasher = SipHasher13::new_with_keys(seed, !seed);
        hasher.write(plaintext.as_bytes());
        format!("{:032x}", hasher.finish128().as_u128())
    }
}

/// Das feste Etikett, über das [`seed_marker`] aus dem Seed einen stabilen, opaken Marker ableitet.
/// Es ist **kein** Konto- und **kein** Klartextwert — nur der Aufhänger, der den seed-gekeyten Hash
/// von den Konto-Token unterscheidet, die derselbe Obfuskator aus IBANs bildet.
const SEED_MARKER_LABEL: &str = "seed-marker";

/// Leitet aus dem Seed das **opake Kennzeichen der Tokenisierung** ab — den Wert, über den `auswerten`
/// prüft, ob Fenster und Eigene-Konten-Config aus **demselben** Seed tokenisiert wurden
/// ([ADR-0010](../../../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)).
///
/// Es ist bewusst **nicht** der Seed: der seed-gekeyte Hash über [`SEED_MARKER_LABEL`] ist stabil und
/// vergleichbar, gibt den Seed aber nicht preis.
///
/// **Er wird abgeleitet, bevor er die Platte berührt.** Die `obfuskiert`-Tabelle legt diesen Marker ab,
/// nicht den Seed — denn ein Vergleich braucht nur Vergleichbarkeit, und was nie gespeichert wird, kann
/// aus einer weitergegebenen Datenbankdatei auch nicht zurückgewonnen werden. Läge dort der Seed, wäre
/// jedes `KONTO_…`-Token gegen eine Klartextliste durchprobierbar und die Pseudonymisierung für jeden
/// umkehrbar, der die Datei hat. Der Seed bleibt so im Prozess, wo
/// `.rules/common/privacy-invarianten.md` („Der Seed bleibt lokal") ihn haben will.
pub(crate) fn seed_marker(seed: u64) -> String {
    SeededObfuscator.obfuscate(seed, SEED_MARKER_LABEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 42;

    #[test]
    fn gleicher_seed_und_klartext_ergeben_denselben_ersatzwert() {
        let obfuscator = SeededObfuscator;
        assert_eq!(
            obfuscator.obfuscate(SEED, "DE89370400440532013000"),
            obfuscator.obfuscate(SEED, "DE89370400440532013000"),
            "der Seam ist deterministisch: gleiche Eingabe → gleicher Ersatzwert"
        );
    }

    #[test]
    fn verschiedene_seeds_ergeben_verschiedene_ersatzwerte() {
        let obfuscator = SeededObfuscator;
        assert_ne!(
            obfuscator.obfuscate(SEED, "DE89370400440532013000"),
            obfuscator.obfuscate(SEED + 1, "DE89370400440532013000"),
            "ein anderer Seed muss (praktisch immer) einen anderen Ersatzwert ergeben"
        );
    }

    #[test]
    fn verschiedene_klartexte_ergeben_verschiedene_ersatzwerte() {
        let obfuscator = SeededObfuscator;
        assert_ne!(
            obfuscator.obfuscate(SEED, "DE89370400440532013000"),
            obfuscator.obfuscate(SEED, "DE11520513735120710131"),
            "verschiedene Klartexte müssen (praktisch immer) verschiedene Ersatzwerte ergeben"
        );
    }

    #[test]
    fn der_rohe_klartext_steckt_nicht_im_ersatzwert() {
        let obfuscator = SeededObfuscator;
        let klartext = "DE89370400440532013000";
        let ersatz = obfuscator.obfuscate(SEED, klartext);
        assert!(
            !ersatz.contains(klartext),
            "der Ersatzwert darf den rohen Klartext nicht enthalten"
        );
        // 128 Bit als Hex = 32 Zeichen, nicht-leer.
        assert_eq!(
            ersatz.len(),
            32,
            "der Ersatzwert ist 128 Bit breit (32 Hex-Zeichen)"
        );
    }
}
