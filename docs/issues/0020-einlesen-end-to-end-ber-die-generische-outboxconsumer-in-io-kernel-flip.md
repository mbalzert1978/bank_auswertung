---
id: "0020"
title: einlesen end-to-end über den generischen OutboxConsumer in io (Kernel-Flip)
status: closed
---

# einlesen end-to-end über den generischen OutboxConsumer in io (Kernel-Flip)

## Parent

Umsetzung von [ADR-0011](../adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
(„Der Vertical Slice ist eine reine Pipeline — die Outbox-Mechanik lebt in `io`"),
Wayfinder-Planungskarte
[#8](https://github.com/mbalzert1978/bank_auswertung/issues/8).

## What to build

Der `einlesen`-Slice läuft **end-to-end über eine generische `OutboxConsumer<H: Handler>`
im neuen `io`-Crate** statt über die slice-eigene `ConsumeBridge`. Ein
`ReadTransactions`-Envelope geht durch `io` (Envelope extrahieren → slice-gelieferter
`CommandDecoder` mit nackten Strings → `handle`), staged das Aggregat und publiziert
`Ingested`; ein **terminaler Fehler** wird über das `Display` des nackten Domänen-Fehlers
**redigiert dead-lettered**. Danach ist der `einlesen_adapter` **restlos outbox-frei** und
bindet nur noch `application_kernel`.

Das ist der **entschiedene Umbau aus ADR-0011 als eine vertikale Scheibe** (kein separates
Fundament-Ticket): der Kernel-Schnitt aus ADR-0009 **flippt** — `application_kernel` wird
der `io`-sichtbare Slice↔`io`-Vertrag (bindet `_adapter` **und** `io`), `shared_kernel`
komplett `io`-privat (bindet nur `io`), und `application_kernel` wird durch Auflösung von
`TypedOutboxHandler` **dependency-frei**. Die Pipeline-Garantien aus
[ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md) (atomar, at-least-once,
wiederaufsetzbar; Datenschutz-Redaktion) bleiben unberührt maßgeblich — nur die Verortung
der Outbox-Mechanik ändert sich.

`obfuskierung` bleibt in dieser Scheibe **unverändert** auf seiner eigenen `ConsumeBridge`
(nutzt kein `TypedOutboxHandler`, bindet `shared_kernel` weiter direkt) — die Suite bleibt
grün. Die neue Enforcement-Regel „`_adapter` bindet kein `shared_kernel`" wird deshalb
**noch nicht** hinzugefügt (sie kann erst greifen, wenn auch `obfuskierung` migriert ist);
gelockert wird nur die ADR-0009-Regel „nur `_adapter` bindet `application_kernel`" auf
„`_adapter` **und** `io`".

Der Command-Decoder-Vertrag aus ADR-0011 (Shape Y — der Slice nennt am Eingang **keinen**
Outbox-Typ):

```rust
trait CommandDecoder {
    type Command;
    fn decode(partition_key: &str, content: &str) -> Option<Self::Command>;
}
```

## Acceptance criteria

- [ ] Neues `io`-Crate mit generischer `OutboxConsumer<H: Handler>` (`impl
  shared_kernel::OutboxHandler`): extrahiert die Envelope und ruft den slice-gelieferten
  `CommandDecoder` mit nur `partition_key`/`content` als `&str` (Shape Y), dispatcht an den
  Handler.
- [ ] Jeder Handler-Fehler ist terminal → Dead-Letter über das **`Display`** des Fehlers
  (redigiert, **nie `Debug`**); `OutboxError` wird nur `io`-intern gebaut; Retry entsteht
  ausschließlich `io`-intern (nie aus einem Handler-Fehler).
- [ ] Die eine `UnitOfWork` in der `OutboxConsumer`: Consume-Ack + staged Store + staged
  Publish atomar; der Ausgang läuft über `application_kernel::Publisher`, den `io` an
  `shared_kernel::OutboxPublisher` bindet.
- [ ] `CommandDecoder`-Trait in `application_kernel`; `TypedOutboxHandler` aufgelöst (Logik
  in die `OutboxConsumer` gezogen) → `application_kernel` wird dependency-frei.
- [ ] `einlesen_adapter`: eigener `CommandDecoder`, Publisher-Bridge auf
  `application_kernel::Publisher` umgehängt, `ConsumeBridge` + jede `shared_kernel`-Nutzung
  entfernt → bindet nur `application_kernel`.
- [ ] `io` verdrahtet `einlesen`s Handler + Decoder in die `OutboxConsumer` (Auflösung per
  `message_type`).
- [ ] `einlesen_specs`/TestApi auf **Handler-Ebene** über den `application_kernel`-Vertrag
  (Command/Strings rein → staged Aggregat + publiziertes `Ingested` bzw. Domänen-Fehler);
  keine `OutboxMessage`/`OutboxError`-Asserts mehr.
- [ ] `io`-Integrationstest: `ReadTransactions`-Envelope → `OutboxConsumer` → echtes
  `einlesen` → publiziertes `Ingested`; terminaler Fehler → redigiertes Dead-Letter.
- [ ] `architecture`-Regel gelockert: `io` darf `application_kernel` binden; volle Suite
  grün (`cargo build`/`test`/`clippy --all-targets`/`fmt --check` + `cargo test -p
  architecture`).
- [ ] Der echte SQLite-/`rusqlite`-Teil des `io`-Crate bleibt draußen (separater `io`-Effort,
  Build-Order Schritt 5) — die Consumer-Maschinerie läuft gegen Fakes.

## Blocked by

None - can start immediately.
