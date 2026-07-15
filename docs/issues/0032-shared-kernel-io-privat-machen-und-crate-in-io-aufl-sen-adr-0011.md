---
id: "0032"
title: shared_kernel io-privat machen und Crate in io auflösen (ADR-0011)
status: closed
---

# shared_kernel io-privat machen und Crate in io auflösen (ADR-0011)

## What to build

`shared_kernel` als **eigene Crate auflösen** und ihren Inhalt **io-privat** in `io`
unterbringen — der Schritt aus [ADR-0011](../adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
der den Kernel-Schnitt flippt: `shared_kernel` (die Outbox-Seams
`OutboxMessage`/`OutboxCommand`/`OutboxError`/`OutboxHandler`/`OutboxPublisher`/`UnitOfWork`)
wird **komplett io-privat**, `application_kernel` bleibt der einzige slice↔io-Vertrag.

**Warum das jetzt trägt (an den Kanten geprüft):** `shared_kernel` wird heute **ausschließlich
von `io`** gebunden und benutzt (`io/src` + `io/tests`). Die Slice-Adapter binden
`application_kernel`, **nicht** `shared_kernel`; die Specs hängen nur am eigenen Adapter
(sie erwähnen `shared_kernel` nur noch in einem Kommentar). Eine separate, dependency-freie
Contract-Crate für einen einzigen Consumer verdient ihren Unterhalt nicht mehr — sie ist ein
Artefakt des ADR-0009-Splits aus der Zeit **vor** `io`.

**Umfang:** die sechs Outbox-Seam-Typen wandern aus `crates/shared_kernel/src/*.rs` in ein
**io-privates** Modul (z. B. `io/common/outbox/` bzw. `io/…`, je Trait weiter eine eigene
Datei); die Crate `shared_kernel` samt `Cargo.toml` entfällt; `io/Cargo.toml` verliert die
`shared_kernel`-Dependency; die `use shared_kernel::…`-Zeilen in `io` (inkl. der beiden
io-Integrationstests) zeigen auf das neue io-interne Modul. Der `architecture`-Test und die
Workspace-`members` verlieren die `shared_kernel`-Crate; die Hook-Globs entsprechend. Kein
Slice-Code ändert sich (die Adapter kannten `shared_kernel` ohnehin nicht).

Rein additiv zu klären beim Umsetzen: ob die Typen io-`pub` (für die io-Integrationstests
sichtbar) oder crate-privat mit `pub(crate)` liegen — die Tests in `io/tests/` sind separate
Crates und brauchen eine sichtbare Fläche; ein schmales `pub`-Modul in `io` genügt.

## Acceptance criteria

- [ ] Die Crate `crates/shared_kernel` existiert nicht mehr; ihre sechs Seam-Typen liegen
      io-privat in `io` (je Trait eine eigene Datei), unverändert in Signatur/Verhalten.
- [ ] `io/Cargo.toml` hat **keine** `shared_kernel`-Dependency mehr; alle `use
      shared_kernel::…` in `io/src` und `io/tests` zeigen auf das neue io-interne Modul.
- [ ] Kein Slice-Crate (`*_domain`/`_adapter`/`_specs`) referenziert `shared_kernel` — vorher
      wie nachher; die Adapter binden weiterhin nur `application_kernel`.
- [ ] Workspace-`members`, der `architecture`-Test und die Hook-Globs kennen `shared_kernel`
      nicht mehr; das **Verhalten** der Guards ist unverändert (gleiche Verstöße scheitern,
      `io` bleibt die einzige Crate mit externen Deps).
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0031](0031-crates-auf-einheitliches-ziel-ordner-layout-umstrukturieren.md) —
beide Umbauten fassen `io` an. 0031 gliedert `crates/io/` zuerst intern nach
`<feature>/`+`common/`; danach löst dieses Issue `shared_kernel` auf dem gesetzten Stand
io-privat in `io` auf. So editiert nie mehr als ein Branch gleichzeitig `io`.
