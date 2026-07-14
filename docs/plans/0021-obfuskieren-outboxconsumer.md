# Workflow-Plan: Issue 0021 — obfuskieren end-to-end über die generische OutboxConsumer in io

Mechanische Migration des `obfuskierung`-Slice auf die in 0020 gebaute `io`-Maschinerie,
**Zeile für Zeile parallel zu `einlesen`**. Der Slice wird eine reine
`Command → Domäne → Response`-Pipeline; Consumer-Rolle/UnitOfWork/Dead-Letter-Taktik leben in `io`.

## Task-Zerlegung (Ausführungsreihenfolge)

| # | Task | Abhängig von | Verifikation |
|---|------|--------------|--------------|
| 1 | Adapter: `ObfuscatedMessage` → `application_kernel::PublishCommand` (+`Command`), public re-export | – | build |
| 2 | Adapter: neues `ObfuscateCommand` + `ObfuscateCommandDecoder` (Shape Y, blank `partition_key`→`None`), `CONSUMED_MESSAGE_TYPE` | – | build |
| 3 | Adapter: `PublisherBridge` von `OutboxPublisher` → `Publisher<ObfuscatedMessage>` umhängen | 1 | build |
| 4 | Adapter: Slice-Handler `ObfuscatingHandler: application_kernel::Handler` (Spiegel `ReadingHandler`) | 2,3 | build |
| 5 | Adapter: `ConsumeBridge` + alle `shared_kernel`-Nutzungen entfernen; `lib.rs`/`Cargo.toml` auf nur `application_kernel` | 1-4 | build + arch |
| 6 | Adapter: `HandleError`/`BookingLookup::Absent`/`RawBookingSource`/`SourceBridge`/Handler-Doku: „absent" = terminal gebrochenes Invariant (Varianten bleiben) | 5 | build |
| 7 | Adapter-Testapi + Fakes: Handler-Ebene (`run` → `HandleError`), `FakeOutboxPublisher` → `FakePublisher` | 1-6 | test |
| 8 | `obfuskierung_specs`: keine `OutboxError`-Asserts mehr; naked `HandleError`; absent = terminal | 7 | test |
| 9 | `io`: `obfuskierung.rs` (`ObfuskierungTransactionalHandler` + `UnitFactory`), `lib.rs`-Modul, Dispatcher-Doku | 1-6 | build |
| 10 | `io`-Integrationstest `obfuskierung_end_to_end.rs`; architecture-Regel „`_adapter` bindet kein `shared_kernel`" | 9 | test + arch |

## Abnahme
Volle Suite grün: `cargo build`/`test`/`clippy --all-targets`/`fmt --check` + `cargo test -p architecture`.
