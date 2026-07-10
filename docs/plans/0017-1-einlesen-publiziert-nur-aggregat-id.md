# Plan 0017-1 — einlesen publiziert nur die Aggregat-ID statt des kompletten Aggregats

Umsetzung von [Issue 0017](../issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md).
Branch: `feat/0017-einlesen-publiziert-nur-aggregat-id`.

## Ausgangslage (verifiziert im Code)

- Der **obfuskieren**-Slice konsumiert bereits einen **ID-Zeiger** aus der Outbox-Hülle, liest das
  rohe Aggregat **per ID** aus dem einlesen-Speicher (`IngestedSourceSeam`/`RawBookingSource`) und
  publiziert nur die ID (`ObfuscatedMessage` = id, kein serde). Die Ziel-Seite ist also **fertig**;
  `BookingId` verweist in seiner Doku explizit auf 0017.
  > **As-built (0017-Review):** Der Zeiger reist im **`partition_key`**, nicht in `message.id` —
  > diese Ausgangslage-Notiz nannte ursprünglich `message.id`; im Zuge von 0017 wurde auf
  > `partition_key` umgestellt (`id` = io-Message-UUID, siehe ADR-0008 „Envelope-Konvention").
- Der **einlesen**-Slice publiziert heute pro Zeile das **komplette abgeflachte Aggregat**:
  `IngestedMessage` trägt alle Felder (serde), `EventSink::publish(&LineAggregate)` reicht das
  Aggregat, `Store::store(&LineAggregate)` staged ohne ID.

Zu bauen ist damit **nur die einlesen-Seite**: eine stabile ID pro Aggregat vergeben, die
Roh-Zeile unter dieser ID persistieren und `Ingested` als **reinen ID-Zeiger** publizieren.

## Designentscheidung

**Der Speicher vergibt die Identität und gibt sie zurück; der Handler publiziert diese ID als
Zeiger.** (Issue-Wortlaut: „vergibt … und schreibt … unter dieser ID" — Vergabe und Ablage
gekoppelt.) Kein separater Generator-Port: die ID entsteht dort, wo die Roh-Zeile abgelegt wird
(io: `INSERT … RETURNING id`; Fake: deterministische Sequenz `ingested-N`). Das spiegelt die
obfuskieren-Konvention (Message trägt die ID im `partition_key`, kein serde-Inhalt).

- Neues Domänen-VO `IngestionId` (opak, wie obfuskierens `BookingId`).
- `Store::store(&LineAggregate) -> IngestionId` (staged unter der vergebenen ID, gibt sie zurück).
- `EventSink::publish(&IngestionId)` (publiziert **nur** die ID, nicht mehr das Aggregat).
- `IngestedMessage` wird zum reinen Zeiger (nur `id`, `partition_key = id`, **kein** `Serialize`) —
  exakt wie `ObfuscatedMessage`.
- Die **Roh-Zeile** (`Line`) wandert weiterhin vollständig in den Speicher-Seam (Tabelle = Wahrheit).

## Aufgaben (in Abhängigkeitsreihenfolge)

### T1 — einlesen_domain
- `value_objects/ingestion_id.rs`: `IngestionId(String)`, `new(impl Into<String>)`, `as_str()`; opak.
- `value_objects/mod.rs` + `lib.rs`: `IngestionId` exportieren.
- `ports/store.rs`: `store(&self, aggregate) -> IngestionId`; Doku anpassen (vergibt + gibt ID zurück).
- `ports/event_sink.rs`: `publish(&self, id: &IngestionId)`; Doku (publiziert ID-Zeiger).
- `handler/mod.rs`: `let id = self.store.store(&aggregate); self.event_sink.publish(&id);`; Doku.
- **Verifikation:** `cargo build -p einlesen_domain`.

### T2 — einlesen_adapter (Bridges/Seam/Message)
- `abstractions/store_seam.rs`: `store(&self, line: &Line) -> String`; Doku (gibt vergebene ID zurück).
- `bridges/store_bridge.rs`: ID vom Seam holen, in `IngestionId` wrappen.
- `bridges/ingested_publisher.rs`: `publish(&self, id: &IngestionId)` → `IngestedMessage::from_id`.
- `commands/ingested_message.rs`: nur `id`, `partition_key = id`, **kein** `Serialize`, `from_id`.
- `commands/mod.rs`: Doku (ausgehend nur noch Dispatch/ID, kein serde).
- **Verifikation:** `cargo build -p einlesen_adapter`.

### T3 — einlesen_adapter (Fakes/TestApi)
- `testapi/fakes/fake_store_seam.rs`: `(id, Line)`-Paare halten, ID `ingested-{n}` vergeben, zurückgeben;
  Beobachter `lines()`, `ordering_accounts()`, `stored_ids()`.
- `testapi/read_test_api.rs`: `stored_ids()` durchreichen; Doku.
- **Verifikation:** `cargo build -p einlesen_adapter --all-targets`.

### T4 — einlesen_specs
- Happy-Path & Fan-out: `published_partition_keys()`/`stored_ids()` sind jetzt die vergebenen IDs
  (`ingested-1` …); belegen, dass **publizierte ID == gespeicherte ID** (Zeiger == Wahrheit) und die
  Message **nur** die ID trägt.
- Feld-Treue-/Redaktions-/Präsenz-/Währungs-Specs: unverändert (weiter über `stored_lines`).
- **Verifikation:** `cargo test -p einlesen_specs`.

### T5 — Gesamtverifikation
- `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
  `cargo test -p architecture` — alle grün.

## Abnahmekriterien (aus dem Issue)
- [ ] Stabile, laufübergreifend eindeutige ID pro Aggregat; Roh-Zeile unter dieser ID persistiert.
- [ ] `Ingested` trägt **nur** die ID (keine abgeflachten Felder) — über Fakes belegt.
- [ ] Lesbarer einlesen-Speicher-Port liefert per ID (bereits via `IngestedSourceSeam`); keine
      adapter→adapter-Kante.
- [ ] obfuskieren konsumiert die schlanke Message (bereits gebaut); Specs bleiben Black-Box.
- [ ] `architecture`-Test + build/test/clippy/fmt grün.
