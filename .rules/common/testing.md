# Testkultur

Maßgeblich: [ADR-0002](../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md),
[ADR-0008](../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md). Grundsatz: **die
Domäne validiert nicht — sie verlangt.**

## Keine Format-Mechanik auf Ablehnung testen

Format-Dekodierung (deutsche Zahl/Datum) ist `io`s Job hinter `Money`/`Calendar`. Ablehnung
von Format-Mechanik testet `io`/Kernel, nicht den Slice.

Do:

```rust
// unbekannte Währung ist eine Domänen-Entscheidung → Slice-Verhalten, testbar
read_test_api().mit_währung("XYZ").run().expect_err("unbekannte Währung");
```

Don't:

```rust
// vorzeichenloser Betrag / unlesbares Datum = io-Mechanik, gehört nicht in die Spec
read_test_api().mit_datum("kaputt").run().expect_err("unlesbares Datum");
```

## Presence-Enforcement ist Slice-Verhalten → testen

Der `_adapter` (ACL) lehnt ein fehlendes Pflichtfeld ab und lässt den Batch **terminal**
scheitern — redigiert. Das ist beobachtbar, also testen die Specs es.

Do:

```rust
let err = read_test_api().ohne_pflichtfeld("betrag").run().unwrap_err();
assert!(!format!("{err}").contains(roher_wert)); // Batch-atomar + redigiert
```

Don't:

```rust
assert_eq!(RawLine::try_from(&record).is_err(), true); // Konstruktor-Interna statt Verhalten
```

## Verhalten durch die `*TestApi`, nicht Interna

Verhalten läuft durch die Fluent `*TestApi` gegen Fakes; die Domäne ist transitiv abgedeckt.
Keine Unit-Tests, die Konstruktor-/Algorithmus-Interna festnageln.

Do:

```rust
read_test_api().mit_zeile(gültige_zeile()).run().expect("Command + gestagetes Aggregat");
```

Don't:

```rust
assert_eq!(Amount::with_direction(5, Richtung::Eingang).minor(), 5); // Interna, churnt weg
```

## `*_specs` sind reine Black-Box

Nur über die eine öffentliche Adapter-Oberfläche (TestApi + DTOs + der nackte Domänen-Fehler
des Slice). Keine Outbox-Typen, keine DTO-Konstruktor-Tests.

Do:

```rust
use obfuskierung_adapter::{ObfuscateTestApi, IngestedRow, ObfuscationError}; // öffentliche Fläche
```

Don't:

```rust
use obfuskierung_adapter::internal::ConsumeBridge; // Interna in der Spec
```
