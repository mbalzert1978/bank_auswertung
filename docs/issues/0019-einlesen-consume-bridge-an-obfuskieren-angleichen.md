---
id: "0019"
title: einlesen-Consume-Bridge an obfuskieren angleichen (Command direkt statt leerem serde-Roundtrip)
status: closed
---

# einlesen-Consume-Bridge an obfuskieren angleichen (Command direkt statt leerem serde-Roundtrip)

**Typ:** AFK

> Aufräum-Folge aus [Issue 0018](0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md):
> dort wurde der `einlesen`-Command zum reinen **Auslöser** (feldlose Payload, Quelle
> im Envelope-`partition_key`), die Consume-Bridge behielt aber bewusst
> (minimal-diff) das alte Nutzlast-Gerüst. Dieses Issue zieht das nach, sodass beide
> Pipeline-Consumer denselben Konsum-Pfad haben.

## Was zu bauen ist

Beide Pipeline-Consumer bekommen ein **Command-Objekt per dispatch** reingereicht —
das ist die gemeinsame Invariante ([ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):
der Handler arbeitet auf Domänen-Command-Typen). Was das Command **trägt**, ist ein
Domänen-Detail: `obfuskieren`s `ObfuscateBooking` trägt eine **ID** (aus dem
`partition_key`), `einlesen`s `ReadTransactions` trägt (künftig) eine **Config**.
Die einzige Aufgabe der Consume-Bridge ist in beiden Fällen dieselbe:
`OutboxMessage` (Envelope) → **Command bauen** → `handler.handle(command)`.

`obfuskieren` macht das **direkt**: `impl OutboxHandler` implementiert den Seam, baut
das Command inline aus dem Envelope und mappt den Handler-Fehler inline auf die
Outbox-Disposition — **kein serde, kein Wire-DTO**. `einlesen` fährt nach 0018 noch das
alte Gerüst: `TypedOutboxHandler` + serde-DTO `ReadTransactionsMessage`, dessen
`deserialize` seit 0018 nur noch ein leeres `{}` erzeugt und wegwirft. Das ist
vestigiales Gerüst, kein vorausschauendes Werkzeug — der Bau-Schritt ist identisch mit
`obfuskieren`, nur schwerer verdrahtet.

Dieses Issue gleicht `einlesen` an `obfuskieren` an: die Consume-Bridge implementiert
`OutboxHandler` direkt, baut das Command (`ReadTransactions::new(RunConfiguration)`)
unmittelbar und mappt `ReadError` inline auf `OutboxError::Warning`. Das entfernte
Gerüst (`TypedOutboxHandler`-Anbindung, `ReadTransactionsMessage`, der
`TerminalError`-Newtype, die serde-Bindung) ist **additiv wiedereinführbar**, falls die
`RunConfiguration` beim io-Schritt einen echten Wire-Inhalt bekommt, der reicher ist als
der `partition_key` — dieselbe YAGNI-Logik, mit der 0018 die serde-Derives von
`RawRecord` entfernt hat.

- **`einlesen_adapter`:** ConsumeBridge → `impl OutboxHandler` direkt; `ReadError` inline
  redigiert auf `OutboxError::Warning` (Vorbild `obfuskieren`s `HandleError`-Mapping).
  `ReadTransactionsMessage`, der `TerminalError`-Newtype und die serde-Bindung entfallen;
  der Slice bindet danach kein serde mehr (`IngestedMessage` war schon serde-frei).
- **TestApi:** `run` baut die `OutboxMessage` ohne serialisierten `{}`-Inhalt (die Quelle
  reist weiter nur als `partition_key`), verdrahtet die Bridge unverändert in die echte
  Kette; Beobachter und `with_*`-Fläche bleiben gleich.

## Abnahmekriterien

- [ ] `einlesen`s ConsumeBridge implementiert `OutboxHandler` direkt (Envelope →
      `ReadTransactions::new(RunConfiguration)` → `handle`); `TypedOutboxHandler` wird
      im Slice nicht mehr angebunden.
- [ ] `ReadTransactionsMessage`, der `TerminalError`-Newtype und die serde-Bindung in
      `einlesen_adapter` sind entfernt; der Slice bindet **kein serde** mehr.
- [ ] `ReadError` wird inline redigiert auf `OutboxError::Warning` gemappt (wie
      `obfuskieren` `HandleError` mappt); das batch-atomare Dead-Letter-Verhalten bleibt
      unverändert (Beschaffungs-/Präsenz-/Deutungs-Fehler → `Warning`).
- [ ] Die TestApi-`run` baut die `OutboxMessage` ohne serialisierten `{}`-Inhalt; die
      Slice-Specs bleiben Black-Box über die Fluent-TestApi und unverändert grün
      (Happy-Path, Fan-out, Redaktion, fehlendes Pflichtfeld, unbekannte Währung, nicht
      beschaffbare Quelle).
- [ ] Nur `einlesen_*`-Crates berührt (`obfuskierung`, beide Kernel, `architecture`, Hook
      unangetastet); `cargo build`/`test`/`clippy --all-targets`/`fmt --check` und
      `cargo test -p architecture` grün.

## Blockiert durch

- Baut auf [0018](0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)
  auf (das den Command zum feldlosen Auslöser gemacht und das hier zu entfernende Gerüst
  bewusst stehen gelassen hat).
- Vorbild: der Consume-Pfad des `obfuskieren`-Slice (direkter `OutboxHandler`-`impl`,
  Command inline, serde-frei).
