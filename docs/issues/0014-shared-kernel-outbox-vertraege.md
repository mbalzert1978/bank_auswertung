---
id: "0014"
title: "shared_kernel: Outbox-Verträge (interface-only)"
status: closed
---

# shared_kernel: Outbox-Verträge (interface-only)

**Typ:** AFK

> Neu aus [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md)
> („`shared_kernel` bekommt die Outbox"). Deckt die in der PRD unter
> „Event-getriebene Pipeline aus Slices" geforderten Messaging-/Outbox-Verträge ab,
> die bislang kein Issue trug.

> **Umgesetzt** — in der Gestalt **verfeinert durch
> [ADR-0009](../adr/0009-kernel-split-und-idiomatische-outbox-vertraege.md):** die
> unten in C#-naher Form gelisteten Verträge sind Rust-idiomatisch
> (`Result<(), OutboxError>` statt `HandleOutcome`/`Severity`, `publish` statt
> `add_to_outbox`, `const MESSAGE_TYPE` statt `message_type()`, `TypedOutboxHandler`
> statt `OutboxHandlerBase`), und der eine `shared_kernel` ist in `shared_kernel`
> (io-facing Outbox-Seams) + `application_kernel` (slice-interne
> `Handler`/`Command`/`Publisher`/`TypedOutboxHandler`) geschnitten. Der Text unten
> bleibt als Ausgangs-Scope (Historie) stehen.

## Was zu bauen ist

Der `shared_kernel` bleibt **dünn und interface-only** (nur `std`, keine externen
Deps, keine Value Objects — [ADR-0007](../adr/0007-modularer-monolith-feature-slices.md)),
wird aber um die **Outbox-Verträge** erweitert, auf denen die event-getriebene
Pipeline (`einlesen → obfuskieren → auswerten → report`) aufsetzt. Vorbild ist die
interne C#-Referenz unter `examples/` (`Common/Abstractions` + `Outbox/Adapter`).

Bereits vorhanden (behalten): `Handler` (mit assoziierten `Command`/`Response`/
`Error`), `Command`, `PublishCommand`, `Publisher`. Neu hinzuzufügen als Traits/
Marker/Enums (keine Implementierung, kein IO):

- **`OutboxMessage`** — Envelope: Id / Type / PartitionKey / Content / Zeit / Attempt.
- **`OutboxCommand`** — Wire-Marker mit `message_type` + `partition_key`.
- **`OutboxPublisher`** — legt ein Wire-DTO in die Outbox (`add_to_outbox`).
- **`OutboxHandler`** — `handle(OutboxMessage) -> HandleOutcome`.
- **`HandleOutcome`** — Summentyp `Handled` / `Retry` / `Faulted`.
- **`Severity`** — Fehlerschwere für das Outcome-Mapping.
- **`UnitOfWork`** — atomarer Commit (eigene Tabelle **und** Outbox-Message in einer
  Transaktion).
- Ein **`OutboxHandlerBase`-Äquivalent** (Consume-Bridge-Muster) als Vertrag/Default,
  soweit ohne IO ausdrückbar.
- Eine dokumentierte **JSON-Konvention** für `OutboxMessage.content` ↔ Domänen-Command.

Diese Verträge sind **Adapter-Niveau** — die blinde Domäne nutzt sie nie. `Response`/
`Error` tragen bewusst **keine** Trait-Schranke im Kernel (das Mappen `Error` →
`HandleOutcome` lebt adapterspezifisch in der Consume-Bridge, ADR-0008).

## Abnahmekriterien

- [ ] `OutboxMessage`, `OutboxCommand`, `OutboxPublisher`, `OutboxHandler`,
      `HandleOutcome` (`Handled`/`Retry`/`Faulted`), `Severity`, `UnitOfWork` und das
      `OutboxHandlerBase`-Äquivalent existieren als reine Interfaces/Marker/Enums im
      `shared_kernel`.
- [ ] Die JSON-Konvention für `OutboxMessage.content` ↔ Domänen-`Command` ist
      dokumentiert (Doc-Kommentar/Modul-Doku), ohne konkrete Serde-Impl im Kernel.
- [ ] `shared_kernel` bleibt **dependency-frei** (nur `std`): kein `serde`, kein
      `thiserror`, keine externen Crates — der `architecture`-Test bleibt grün und der
      PreToolUse-Hook schlägt nicht an.
- [ ] Die blinde Domäne bindet den `shared_kernel` **nicht** (unverändert durch den
      `architecture`-Dependency-Richtungs-Test belegt).
- [ ] `cargo build`/`test`/`clippy --all-targets` und `cargo fmt --check` grün.

## Blockiert durch

Keine — kann sofort starten (`shared_kernel` existiert bereits, wird nur erweitert).
