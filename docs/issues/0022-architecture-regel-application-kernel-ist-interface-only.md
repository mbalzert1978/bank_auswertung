---
id: "0022"
title: "architecture-Regel: application_kernel ist interface-only (schließt die shared_kernel-Lücke)"
status: open
---

# architecture-Regel: application_kernel ist interface-only (schließt die shared_kernel-Lücke)

## Kontext

Rest-Lücke aus dem ADR-0011-Contract-Schritt (Planungskarte #8). Der eigentliche
Contract-Schritt — Ban-Regel `adapter_binden_kein_shared_kernel`, Specs auf den
`application_kernel`-Vertrag umgestellt, alle `<slice>_adapter`/`_specs`
`shared_kernel`-frei — ist bereits durch die gemergten Issues **0020** (Kernel-Flip,
einlesen) und **0021** (obfuskieren) umgesetzt und auf `main` committet.

Prüft man ADR-0011/AC #3 („kein Crate außer `io` bindet `shared_kernel`") crate-Typ für
crate-Typ gegen [architecture/tests/dependency_direction.rs](../../architecture/tests/dependency_direction.rs),
ist heute **alles bis auf eine Kante** durch einen Test erzwungen:

| Crate-Typ | erzwungen durch |
|---|---|
| `*_domain` | `domaenen_crates_sind_dependency_frei` (dep-frei) |
| `*_adapter` | `adapter_binden_kein_shared_kernel` |
| `*_specs` | `specs_haengen_nur_am_eigenen_adapter` (nur eigener Adapter) |
| `shared_kernel` | `shared_kernel_ist_interface_only` (dep-frei) |
| `io` | darf `shared_kernel` binden (soll es) |
| **`application_kernel`** | **nichts** — `application_kernel_binden_nur_adapter_und_io` prüft nur, *wer* ihn bindet, nicht *was er bindet* |

`application_kernel` ist heute faktisch dep-frei
([crates/application_kernel/Cargo.toml](../../crates/application_kernel/Cargo.toml),
leeres `[dependencies]`), aber **kein Test** hält es davon ab, morgen `shared_kernel`
(oder irgendeine Fremd-Crate) zu binden. Das ist die einzige verbliebene Kante.

## What to build

Eine neue Regel `application_kernel_ist_interface_only` in
`architecture/tests/dependency_direction.rs`, **exakt gespiegelt** zur bestehenden
`shared_kernel_ist_interface_only`: sie assertiert crate-level über `cargo_metadata`,
dass `application_kernel` **dependency-frei** ist. Dep-frei ⇒ kann `shared_kernel` nicht
binden — die Lücke ist geschlossen, und zwar härter als eine reine `shared_kernel`-Ban:
`application_kernel` ist ein *thin, interface-only* Kernel (ADR-0009/0011) und soll gar
keine Dependency tragen.

Bewusst **eng** statt breit — die gespiegelte Interface-only-Regel deckt genau das eine
ungesicherte Crate ab; warum keine globale „nur `io`"-Sammelregel, regelt das
Abnahmekriterium „Keine Überlappung". `architecture` bleibt außen vor: der von allen
Regeln ausgenommene Validator (Test-Crate außerhalb von `crates/`, kein
Laufzeit-/Privacy-Pfad).

## Acceptance criteria

- [ ] Neue `#[test] application_kernel_ist_interface_only` in
  `architecture/tests/dependency_direction.rs`, crate-level über `cargo_metadata`, im
  Stil (deutscher Spezifikationssatz als lebende Doku) und **1:1 gespiegelt** zu
  `shared_kernel_ist_interface_only`.
- [ ] Die Regel schlägt fehl, sobald `application_kernel` **irgendeine** Dependency bindet
  (womit insbesondere eine `shared_kernel`-Bindung ausgeschlossen ist).
- [ ] Gegenprobe im Grünen: `application_kernel` ist dep-frei, die Regel ist ohne Änderung
  an einem Manifest erfüllt.
- [ ] Keine Überlappung: die bestehenden Regeln (`adapter_binden_kein_shared_kernel`,
  `specs_haengen_nur_am_eigenen_adapter`, `domaenen_crates_sind_dependency_frei`,
  `shared_kernel_ist_interface_only`) bleiben unverändert; **keine** breite „nur
  `io`"-Sammelregel wird hinzugefügt.
- [ ] Volle Suite grün: `cargo build`, `cargo test`, `cargo clippy --all-targets`,
  `cargo fmt --check` **und** `cargo test -p architecture`.
- [ ] Kein Laufzeitverhalten der Slices/`io`-`OutboxConsumer` geändert, PreToolUse-Hook
  (`enforce-dependency-direction.py`) unverändert, keine neue externe Dependency.

## Blocked by

None — can start immediately. Baut auf den bereits gemergten Issues
**0020** ([0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md))
und **0021** ([0021-obfuskieren-end-to-end-ber-die-generische-outboxconsumer-in-io.md](0021-obfuskieren-end-to-end-ber-die-generische-outboxconsumer-in-io.md))
auf, die die Adapter/Specs bereits `shared_kernel`-frei gemacht haben; deren Code liegt auf
`main`, daher ist diese Rest-Lücke jetzt greifbar.
