---
id: "0049"
title: "Persistenz-Stack festlegen: rusqlite (synchron), kein sqlx"
status: open
---

# Persistenz-Stack festlegen: rusqlite (synchron), kein sqlx

**Typ:** AFK

## What to build

Ein neues ADR (Fortsetzung der Reihe, **ADR-0014**), das die eine ripple-schwere io-Weiche festzieht,
**bevor** irgendein realer Store-Code entsteht: die reale `io`-Persistenz läuft über **rusqlite** und
bleibt **synchron**.

Begründung, die das ADR festhält: die gesamte Outbox-Spine ist bewusst sync — `Handler::handle`,
`TransactionalHandler::handle`, `UnitOfWork::commit`, `OutboxHandler::handle` sind allesamt sync `fn`,
kein async-Runtime ([ADR-0003](../adr/0003-rust-als-sprache.md)). `sqlx` ist async-first; es
hereinzuziehen hieße **entweder** die komplette Kette durch `application_kernel` und alle Slices auf
async umzubauen (breiter Ripple) **oder** mit `block_on` auf einer Runtime zu blockieren (Hack).
**rusqlite** fällt hinter die bestehenden sync-Traits, **ohne eine einzige Signatur zu ändern**.

Der Widerspruch CLAUDE.md↔Code ist mitzuziehen: CLAUDE.md nennt heute an mehreren Stellen `sqlx` als
künftigen io-Adapter. Diese Verweise werden auf rusqlite korrigiert, damit kein Doku-Ort mehr async-io
behauptet.

**Reine Entscheidung + Doku — kein Adapter-Code.** Blockiert die transaktionale Backbone und die
Slice-Stores ([0052](0052-sqlite-outbox-broker-und-unitofwork-real-rusqlite.md),
[0053](0053-einlesen-store-real-und-end-to-end.md)); die persistenzfreien Schnitte
([0050](0050-money-calendar-decode-seams-real.md),
[0051](0051-einlesen-csv-camt-v8-quelle-real-hinter-source-seam.md),
[0054](0054-config-loader-toml-fenster-config.md)) hängen **nicht** daran.

## Acceptance criteria

- [ ] Ein akzeptiertes **ADR-0014** hält fest: `io`-Persistenz = rusqlite, die Outbox-Spine bleibt
      **synchron**, sqlx ist verworfen — samt Begründung (async-Ripple vs. ADR-0003).
- [ ] Alle `sqlx`-Verweise in CLAUDE.md sind auf rusqlite gezogen; kein Doku-Ort behauptet mehr
      async-`io`.
- [ ] Reine Doku-/Entscheidungs-Änderung — **kein Code-Verhalten ändert sich**.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
      `cargo test -p architecture` und `cargo doc --no-deps --workspace` sind grün.

## Blocked by

None - can start immediately.
