# Performance

Optimieren heißt **erst messen, dann ändern**. Die folgenden Regeln decken die
Iterator-Nutzung, zwei Mess-Werkzeuge und das Release-Profil ab.

## Zero-Cost-Iteratoren — nicht unnötig collecten

Rusts Zero-Cost-Iterator-Abstraktionen nutzen und Zwischenergebnisse nicht unnötig in einen
`Vec` sammeln.

Do:

```rust
fn summe_der_geraden_quadrate(zahlen: &[u64]) -> u64 {
    zahlen.iter().filter(|&&n| n % 2 == 0).map(|&n| n * n).sum() // eine Pipeline, keine Allokation
}
```

Don't:

```rust
fn summe_der_geraden_quadrate(zahlen: &[u64]) -> u64 {
    let geraden: Vec<u64> = zahlen.iter().filter(|&&n| n % 2 == 0).copied().collect();
    let quadrate: Vec<u64> = geraden.iter().map(|&n| n * n).collect();
    quadrate.iter().sum() // zwei überflüssige Heap-Allokationen, die der Iterator vermeidet
}
```

## CPU-Profiling mit cargo-flamegraph

Vor einer Performance-Optimierung mit `cargo-flamegraph` den heißen Pfad messen, statt ihn
zu raten.

Do:

```rust
// $ cargo flamegraph --bin auswerten   → weist den Hotspot am Release-Binary nach
fn heisser_pfad(werte: &[u64]) -> u64 {
    werte.iter().copied().sum() // im Flamegraph als Hotspot bestätigt, dann optimiert
}
```

Don't:

```rust
fn selten_aufgerufen() -> u64 {
    aufwendige_handoptimierung() // am kalten Pfad „optimiert" ohne Profil — echter Hotspot bleibt unentdeckt
}
```

## Allokationen mit cargo-heaptrack messen

Die Allokationslast mit `cargo-heaptrack` messen, statt Allokationen zu vermuten.

Do:

```rust
// $ cargo heaptrack --bin auswerten   → zeigt die tatsächlichen Allokations-Hotspots
fn verarbeite(zeilen: &[u64]) -> u64 {
    zeilen.iter().copied().sum() // arbeitet auf dem Borrow, keine Kopie
}
```

Don't:

```rust
fn verarbeite(zeilen: &[u64]) -> u64 {
    let kopie: Vec<u64> = zeilen.to_vec(); // ungemessene „Optimierung": klont die Eingabe — Allokations-Spike
    kopie.iter().copied().sum()
}
```

## LTO und codegen-units = 1 im Release-Profil

Im Release-Profil Link-Time-Optimization aktivieren und `codegen-units = 1` setzen für
maximale Performance. (Diese Regel betrifft `Cargo.toml`, daher als Profil-Ausschnitt statt
als Rust-Code.)

Do:

```toml
[profile.release]
lto = true
codegen-units = 1   # eine Codegen-Einheit ⇒ maximale Cross-Function-Inlining-Chancen
```

Don't:

```toml
[profile.release]
# Defaults belassen: lto = false, codegen-units = 16 — schnellerer Build, aber langsameres Binary
```
