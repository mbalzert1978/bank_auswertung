# Domänenmodell-Konventionen

Maßgeblich: [ADR-0005](../../docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md).
Kernhaltung: **_parse, don't validate_** — einmal am Rand validieren, danach trägt der Typ
den Beweis. Rust-idiomatische Umsetzung: [rust/idiome.md](../rust/idiome.md).

## Value Objects statt Primitiven

Jedes Konzept ist ein Typ, der beim Konstruieren validiert. Kein rohes `String`/`f64`
überschreitet die DTO-Grenze in die Domäne.

Do:

```rust
pub struct Betrag { minor: u64, währung: Währung } // konstruiert = gültig
fn buchen(betrag: Betrag) { /* … */ }
```

Don't:

```rust
fn buchen(betrag: f64, währung: &str) { /* jeder Aufrufer muss selbst validieren */ }
```

## Summentypen statt Flag + optionale Felder

Illegale Kombinationen müssen **unkonstruierbar** sein; erschöpfendes `match` statt
verstreuter `if`.

Do:

```rust
enum Gegenkonto { Eigen(Rolle), Fremd }
match gegenkonto { Gegenkonto::Eigen(r) => …, Gegenkonto::Fremd => … }
```

Don't:

```rust
struct Gegenkonto { ist_eigen: bool, rolle: Option<Rolle> } // {ist_eigen: false, rolle: Some(_)} illegal, aber baubar
```

## `Betrag` vorzeichenlos — das Vorzeichen lebt in `Richtung`

Ein negativer `Eingang` ist unkonstruierbar. Die Domäne summiert keine gemischten Währungen
(der eingehende Adapter garantiert eine einzige).

Do:

```rust
struct Betrag { minor: u64, währung: Währung } // vorzeichenlos
enum Richtung { Eingang, Ausgang }             // Vorzeichen nur hier
```

Don't:

```rust
struct Betrag { wert: i64 } // negativer Eingang wird darstellbar
```

## Deklarativ-funktional

Reine Transformationen über unveränderliche Werte; `map`/`filter`/`fold` statt manueller
Schleifen, `match` statt if/else-Kaskaden.

Do:

```rust
let summe = buchungen.iter().filter(|b| b.ist_ausgang()).map(Buchung::betrag).sum();
```

Don't:

```rust
let mut summe = 0;
for b in &buchungen { if b.ist_ausgang() { summe += b.betrag_wert(); } } // mutable Schleife
```
