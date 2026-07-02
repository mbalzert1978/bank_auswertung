/// Das UBQL-DTO für die zu obfuskierende IBAN am Slice-Rand — der **Eingang** der
/// Obfuskierung. **Normalisiert nur** (Großbuchstaben, ohne Leerzeichen), damit
/// verschiedene Schreibweisen desselben Kontos dasselbe `KONTO_x` erhalten, und
/// weist die leere Eingabe ab ([`Iban::new`] → `None`).
///
/// Bewusst **keine** Prüfsummen-/Strukturvalidierung: Obfuskierung deutet keine
/// Kontonummer, sie braucht nur einen stabilen, normalisierten Schlüssel. Die
/// IBAN-Autorität ist der Eigene-Konten-Slice; vorgelagerte Slices reichen bereits
/// validierte IBANs herein. Ein zweiter mod-97 hier würde den Algorithmus über
/// Slices hinweg duplizieren ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).
///
/// Struktureller Identifikator: leitet **kein** `Serialize`/`Deserialize` ab (der
/// compilergeprüfte `assert_not_impl_any!`-Beleg liegt in `lib.rs`) — eine IBAN
/// geht in den Slice hinein, verlässt die Maschine aber nie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iban(String);

impl Iban {
    /// Baut das DTO aus rohem Text: normalisiert (Großbuchstaben, ohne
    /// Leerzeichen) und liefert `None`, wenn danach nichts bleibt.
    pub fn new(input: impl AsRef<str>) -> Option<Self> {
        let normalized: String = input
            .as_ref()
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect();

        (!normalized.is_empty()).then_some(Self(normalized))
    }

    /// Die normalisierte IBAN (Großbuchstaben, ohne Leerzeichen) — der First-Seen-
    /// Schlüssel, den die Domäne obfuskiert.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
