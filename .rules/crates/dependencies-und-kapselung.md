# Dependency-Richtung & Kapselung

**Unumstößlich** (Clean-Architecture-Einmaleins), verankert in
[docs/prd/0001-bank-auswertung.md](../../docs/prd/0001-bank-auswertung.md),
[ADR-0001](../../docs/adr/0001-hexagonale-architektur.md) und
[ADR-0003](../../docs/adr/0003-rust-als-sprache.md). Dreifach erzwungen: (1) Crate-Ebene,
(2) `architecture`-Test (`cargo test -p architecture`), (3)
`enforce-dependency-direction.py` PreToolUse-Hook (blockt eine nicht sanktionierte externe
`[dependencies]`-Zeile in jedem Crate außer `io` und `architecture`).

## Nur `io` bindet konkrete Fremd-Crates

Abhängigkeiten zeigen nach innen. Die Domäne und die Adapter-Kern-/Mapping-Logik binden
**kein** I/O-Crate (`reqwest`/`rusqlite`/`csv`/…); ein konkretes Fremd-Crate wird nie direkt
außerhalb von `io` `use`-t.

Do:

```toml
# crates/io/Cargo.toml
[dependencies]
csv = "1"        # konkrete Fremd-Crates leben nur hier
rusqlite = "0.31"
```

Don't:

```toml
# crates/einlesen_adapter/Cargo.toml
[dependencies]
csv = "1"        # I/O-Crate im Slice-Adapter — verboten
```

## Fremd-Crate hinter einen Port

Bevor ein `use <fremd_crate>` in ein Nicht-`io`-Crate kommt → **stopp**: erst einen Port
definieren, die konkrete Abhängigkeit in `io` legen, den Port dort erfüllen, andere Layer
konsumieren nur den Port.

Do:

```rust
// Slice-Domäne: Port; io: der csv-Adapter erfüllt ihn
pub trait RawLineSource { fn fetch(&self) -> Result<Vec<RawLine>, SourceError>; }
```

Don't:

```rust
// im Slice-Adapter
let mut rdr = csv::Reader::from_path(pfad)?; // konkrete Abhängigkeit in der Mapping-Logik
```

Erlaubte Ausnahme, die die Regel **nicht** bricht: `serde`/`thiserror`/`static_assertions`
außerhalb der Domäne. Die Leak-Garantie ruht auf dem fehlenden `Serialize` der Egress-Typen,
nicht auf serde-Abwesenheit.

## Slice erreicht fremden Slice nur über dessen `_adapter`

Ein `<slice>_domain` ist blind für andere Slices. Kopplung läuft ausschließlich über die
öffentliche `_adapter`-Oberfläche, nie über ein fremdes `_domain`.

Do:

```rust
use obfuskierung_adapter::ObfuscatedBooking; // öffentliches Adapter-DTO
```

Don't:

```rust
use obfuskierung_domain::ObfuscatedAggregate; // Griff ins fremde _domain
```

## Nur `_adapter` bindet `application_kernel`

`application_kernel` ist der einzige Slice↔`io`-Vertrag. Weder ein `_domain` noch ein
`_specs` bindet ihn.

Do:

```toml
# crates/obfuskierung_adapter/Cargo.toml
[dependencies]
application_kernel = { path = "../application_kernel" }
```

Don't:

```toml
# crates/obfuskierung_domain/Cargo.toml
[dependencies]
application_kernel = { path = "../application_kernel" } # Domäne kennt den Kernel-Vertrag — verboten
```
