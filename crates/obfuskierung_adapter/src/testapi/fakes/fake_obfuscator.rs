use crate::abstractions::ObfuscatorSeam;

/// Fake des [`ObfuscatorSeam`]: **deterministisch** über einen kleinen, stabilen Hash (FNV-1a,
/// seed-gekeyt) statt der echten Random-Bibliothek — plattform-/laufstabil und ohne Krypto-Dep in
/// den Specs. Vertrag erfüllt: gleicher Seed + gleiche Eingabe → gleicher Ersatzwert; verschiedene
/// Eingaben/Seeds → (praktisch immer) verschiedene Ersatzwerte; der rohe Klartext steckt **nicht**
/// im Ersatzwert (16 Hex-Zeichen).
#[derive(Debug, Default)]
pub(crate) struct FakeObfuscator;

impl ObfuscatorSeam for FakeObfuscator {
    fn obfuscate(&self, seed: u64, plaintext: &str) -> String {
        format!("{:016x}", fnv1a_seeded(seed, plaintext))
    }
}

/// FNV-1a (64-bit), mit dem Seed als abweichender Offset-Basis gekeyt — deterministisch und
/// stabil (anders als `std`s `DefaultHasher`, der über Rust-Versionen driften darf).
fn fnv1a_seeded(seed: u64, plaintext: &str) -> u64 {
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    let mut hash = FNV_OFFSET ^ seed;
    for byte in plaintext.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
