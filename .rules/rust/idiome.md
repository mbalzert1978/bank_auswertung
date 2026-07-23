# Rust-Idiome

Die sprach-agnostischen Modellierungsprinzipien stehen in
[common/domaenenmodell.md](../common/domaenenmodell.md); hier ihre Rust-Umsetzung.

## `<slice>_domain` ist dependency-frei

Kein externes Crate in der Domäne — nicht einmal `serde`/`thiserror`. Domänen-Fehler sind
**handgeschriebenes `Display` + `std::error::Error`**, vom Adapter gemappt. (Durchsetzung:
[crates/dependencies-und-kapselung.md](../crates/dependencies-und-kapselung.md).)

Do:

```rust
#[derive(Debug)]
pub enum LineError { AmountInvalid, CurrencyUnknown }
impl std::fmt::Display for LineError { /* … */ }
impl std::error::Error for LineError {}
```

Don't:

```rust
use thiserror::Error;
#[derive(Error, Debug)] // externes Crate in der Domäne
pub enum LineError { /* … */ }
```

## Leak = Compile-Fehler, nicht Review-Versäumnis

An wichtigen Grenzen die stärkste Compiler-Garantie wählen: der
`static_assertions`-„kein `Serialize`"-Guard macht das Einbetten roher Identifikatoren
unkompilierbar (siehe [common/privacy-invarianten.md](../common/privacy-invarianten.md)).

Do:

```rust
assert_not_impl_any!(ObfuscatedBooking: serde::Serialize);
```

Don't:

```rust
// Verlassen auf „daran denken, kein Serialize zu deriven" — kein Compiler-Netz
```

## Newtypes validieren beim Konstruieren

Value Objects als Newtypes, die einmal am Rand parsen; danach kein rohes `String`/`f64` mehr.

Do:

```rust
pub struct NonEmptyString(String);
impl NonEmptyString {
    pub fn new(s: &str) -> Option<Self> { let t = s.trim(); (!t.is_empty()).then(|| Self(t.into())) }
}
```

Don't:

```rust
type Pseudonym = String; // kein Typ-Beweis, überall erneut zu prüfen
```
