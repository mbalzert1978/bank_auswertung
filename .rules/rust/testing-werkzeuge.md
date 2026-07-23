# Test-Werkzeuge (property-based & Fuzzing)

Ergänzt die [Testkultur](../common/testing.md) um zwei Rust-Werkzeuge für Fälle, die
Einzelbeispiele nicht erschöpfen.

## Property-based Tests mit proptest

Komplexe Invarianten mit `proptest` property-basiert über den Eingaberaum testen, statt nur
mit einzelnen Beispielwerten.

Do:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn zweimal_reverse_ist_identitaet(v: Vec<u64>) {
        let original = v.clone();
        let mut w = v;
        w.reverse();
        w.reverse();
        prop_assert_eq!(w, original); // hält über den ganzen Eingaberaum, inkl. leer/Duplikate/groß
    }
}
```

Don't:

```rust
#[test]
fn zweimal_reverse_ist_identitaet() {
    let mut v = vec![1u64, 2, 3];
    let original = v.clone();
    v.reverse();
    v.reverse();
    assert_eq!(v, original); // ein einziger Fall — Randfälle (leer, Overflow, Duplikate) ungetestet
}
```

## cargo-fuzz für sicherheitskritisches Parsing

Sicherheitskritischen Parsing- und Deserialisierungs-Code mit `cargo-fuzz` gegen beliebige
Bytes fuzzen.

Do:

```rust
// fuzz/fuzz_targets/parse_camt.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = camt_v8::parse(data); // muss bei beliebigen Bytes sauber Err liefern — nie paniken/UB
});
```

Don't:

```rust
#[test]
fn parst_beispieldatei() {
    let daten = include_bytes!("../fixtures/wohlgeformt.csv");
    assert!(camt_v8::parse(daten).is_ok()); // nur wohlgeformte Eingabe — bösartige Bytes (Panik/OOM/UB) ungetestet
}
```
