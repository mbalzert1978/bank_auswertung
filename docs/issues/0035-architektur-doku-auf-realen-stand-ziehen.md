---
id: "0035"
title: Architektur-Doku auf den realen Stand ziehen (Prefactor)
status: open
---

# Architektur-Doku auf den realen Stand ziehen (Prefactor)

**Typ:** AFK

## What to build

**Prefactoring vor dem Bau** — „make the change easy, then make the easy change". Die
Architektur-Doku beschreibt einen Stand, den es nicht mehr gibt, und jeder Agent, der eines der
Folge-Tickets greift, liest sie **zuerst** und startet mit falschem Weltbild.

Zwei konkrete Abweichungen:

1. **`shared_kernel` ist als Crate aufgelöst** ([0032](0032-shared-kernel-io-privat-machen-und-crate-in-io-aufl-sen-adr-0011.md),
   ADR-0011) — ihre Outbox-Seams liegen io-privat in `io`. Die Doku führt sie weiterhin als
   existierende Crate und spricht von „zwei Kernel-Crates"; real gibt es **`application_kernel`**
   als einzigen Slice↔io-Vertrag.
2. **Die Pipeline hat einen neuen Schritt.** [ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)
   ist **akzeptiert** und schiebt **`fenstern`** zwischen `obfuskieren` und `auswerten`. Doku und
   Baureihenfolge kennen ihn nicht; die dort genannten neuen Begriffe stehen inzwischen in
   [CONTEXT.md](../../CONTEXT.md).

Zu korrigieren sind Crate-Liste, Pipeline-Darstellung, Baureihenfolge und der Status-/Nächste-
Schritte-Abschnitt. **Reine Doku — kein Code-Verhalten ändert sich.**

## Acceptance criteria

- [ ] Die Doku führt `shared_kernel` nicht mehr als existierende Crate; die genannte Crate-Liste
      entspricht dem, was unter `crates/` plus dem Validator-Crate real liegt.
- [ ] Die Pipeline steht durchgängig als `einlesen → obfuskieren → fenstern → auswerten → report`.
- [ ] Die Baureihenfolge enthält `fenstern` als eigenen Schritt **vor** `auswerten`.
- [ ] ADR-0013 ist als **akzeptiert** referenziert (nicht mehr als Entwurf), und die neuen Begriffe
      verweisen auf CONTEXT.md statt auf eine Liste offener Sprach-Begriffe.
- [ ] Der Status-Abschnitt nennt `fenstern` und den `auswerten`-Split als das, was als Nächstes
      gebaut wird.
- [ ] Keine Code-Änderung; `cargo build`, `cargo test`, `cargo clippy --all-targets`,
      `cargo fmt --check` und `cargo test -p architecture` bleiben unverändert grün.

## Blocked by

Keines — kann sofort starten.
