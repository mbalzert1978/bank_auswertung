---
id: "0031"
title: Crates auf einheitliches Ziel-Ordner-Layout umstrukturieren
status: closed
---

# Crates auf einheitliches Ziel-Ordner-Layout umstrukturieren

## What to build

Alle Crates auf **ein** einheitliches internes Ordner-Schema bringen, an dem sich jede
Datei eindeutig verorten lässt. Heute liegen gleichartige Teile uneinheitlich: Aggregate
stecken in `value_objects/` oder in eigenem `aggregate/`, Kommandos und DTOs sind auf
`commands/` + `dto/` verstreut, Adapter heißen `bridges/`, Handler mal `handler.rs` mal
`handler/mod.rs`, `io` ist flach statt nach Feature gegliedert.

Der Umbau ist **rein intra-crate und mechanisch**: nur Datei-Verschiebungen + Anpassung
der `mod`-Deklarationen/-Pfade (und die eine `Cargo.toml`-Änderung beim Entfernen des
Specs-Stubs). **Keine Crate wechselt ihren Ort** — also keine Workspace-`members`-Änderung,
keine Berührung des `architecture`-Tests oder der `enforce-dependency-direction.py`-Hook-Globs.
Die Slice-Grenzen (Hexagonal-Cut) und alle öffentlichen APIs bleiben unverändert; es wird
keine Verantwortung zwischen Schichten verschoben.

### Ziel-Layout (Soll)

**`<feature>_domain/`**
- `models/` — Request-/Config-Structs (domänenintern)
- `ports/` — nach außen sichtbare Ports (Traits)
- `entities/` — **über Identität** definierte Structs: in der DB abgelegt und **per ID
  auffindbar**. Das schließt Aggregat-Roots ein (auch wenn die ID vom Store vergeben wird
  und nicht als Feld im Struct sitzt) — siehe Entscheidung 6
- `value_objects/` — **über den Wert** definierte Structs: per Wert gefunden, **ohne ID**
- `error.rs` und `handler.rs` auf Top-Level neben `lib.rs` (Handler **nicht** in
  `handler/`-Unterordner) — siehe Entscheidung 4

**`<feature>_adapter/`**
- `models/` — Request-/Response-DTOs (Boundary)
- `abstractions/` — public exponierte Ports (Seams)
- `adapter/` — Adapter, die interne Domänen-Ports implementieren und externes↔internes
  Port-Mapping umsetzen
- `testapi/` — die public Fluent-`*TestApi` + In-Memory-Fakes (bleibt im Adapter, siehe
  Entscheidung 1)
- `handler.rs` auf Top-Level

**`<feature>_specs/`**
- `tests/` — ausschließlich **nackte** Tests (Arrange/Act/Assert), die ihre Fixtures aus
  `fixtures/` beziehen; **kein** `src/lib.rs`-Stub (siehe Entscheidung 3)
- `fixtures/` — die **spec-lokalen** Fixtures/Helfer (Input-Builder, Expected-Builder,
  Setup-Helfer, Test-Consts), heute inline im Testfile (siehe Entscheidung 2)

**`crates/io/`** (bleibt unter `crates/`)
- `<feature>/` — Implementierungen je Feature (`einlesen/`, `obfuskierung/`)
- `common/` — feature-übergreifende Bausteine

**Kernel** — `application_kernel` bleibt `crates/application_kernel` **unverändert** (flach,
je Trait eine Datei; es sind nur adapter-zugewandte Traits, bereits konform, **kein**
`kernel/`-Unterordner). `shared_kernel` bleibt ebenfalls unangetastet: es wird nur von `io`
benutzt und laut ADR-0011 io-privat — das erledigt ein **eigenes 0011-Issue** (Auflösung in
`io`), nicht 0031. Siehe Entscheidung 5.

### Getroffene Entscheidungen

Diese Punkte waren offen und sind **hier entschieden** — das Issue ist damit ohne weitere
Vorklärung abnehmbar:

1. **`testapi/`-Verortung:** die Fluent-`*TestApi` **und die In-Memory-Fakes**
   (`FakeSourceSeam`/`FakeStoreSeam`/… — sie verdrahten die echte Bridge-Kette gegen Fakes)
   bleiben **im `_adapter`** unter `testapi/`; sie sind laut CLAUDE.md Teil der public
   Adapter-Oberfläche.
2. **Spec-lokale Fixtures:** die im Testfile inline liegenden Fixtures/Helfer — Input-Builder
   (`gültiger_rohsatz`/`gültige_zeile`), Expected-Builder (`erwartete_zeile`/
   `erwartete_buchung`), Setup-/Zugriffs-Helfer (`api_mit_zeile`/`einzige_buchung`/`feld`) und
   Test-Consts (`QUELLE`/`SEED`/`EIGENE_IBAN`/…) — wandern nach `<feature>_specs/fixtures/`
   und werden von den Tests **hereingezogen**; `tests/` enthält danach nur **nackte** Tests.
   **Nicht** in die TestApi runtergedrückt: die Expected-Builder sind test-seitige
   Erwartungsdaten (die erwartete Projektion eines Laufs) und gehören nicht in den Adapter.
3. **Specs-Stub:** **kein** `src/lib.rs`-Stub in den Specs-Crates (einheitlich zu
   `einlesen_specs`, das keinen hat). Der Stub in `obfuskierung_specs` wird entfernt; ein
   evtl. `[lib]`-Target in dessen `Cargo.toml` mit. Das `fixtures/`-Modul zieht der Test per
   Modul-Include ein — **kein** Lib-Target nötig.
4. **`error.rs`:** bleibt auf **Top-Level** der Domain-Crate (neben `lib.rs`/`handler.rs`).
   Der terminale Fehler-Summentyp des Slice ist weder Request/Config (`models/`) noch VO
   noch Port — ein eigener Top-Level-Belang.
5. **Kernel-/`io`-Platzierung:** **alles bleibt unter `crates/`**. `crates/io/` bleibt, wo
   es ist (nur **interne** Reorg nach `<feature>/` + `common/`). `application_kernel` bleibt
   `crates/application_kernel` **unverändert** — **kein** `kernel/`-Unterordner: es sind nur
   adapter-zugewandte Traits (je Trait eine Datei, schon konform), eine Gruppierung brächte
   keinen Gewinn. `shared_kernel` ist **ausgenommen** (nur von `io` genutzt, ADR-0011
   io-privat → separates 0011-Issue). Damit bleibt der Umbau vollständig **intra-crate**:
   keine Crate wechselt den Ort, keine Workspace-/Guard-Anpassung.
6. **`entities/`-Kriterium:** Unterscheidung **Identität vs. Wert**. Eine Entity ist über
   ihre Identität definiert — sie wird in der DB abgelegt und ist **per ID auffindbar**; ein
   Value Object ist über seinen Wert definiert und wird **per Wert, ohne ID** gefunden.
   Aggregat-Roots sind damit Entities: `LineAggregate` und `ObfuscatedBooking` werden
   abgelegt und per (Store-vergebener) ID geladen — sie gehören nach `entities/`, nicht in
   `value_objects/`, auch wenn das ID-Feld nicht im Struct sitzt. **Ein ID-Newtype selbst**
   (`IngestionId`, `BookingId`) ist dagegen wert-definiert (zwei gleiche Werte sind gleich)
   und bleibt in `value_objects/` — Entity ist das *identifizierte* Ding, nicht der ID-Wert.

### Ist → Soll je Crate

✓ = bereits konform, bleibt.

| Crate | Ist | Soll |
|---|---|---|
| `einlesen_domain` | `aggregate/line_aggregate.rs` | `entities/line_aggregate.rs`, Ordner `aggregate/` auflösen |
| | `commands/read_transactions.rs` (`ReadTransactions`, `RunConfiguration`) | `models/` |
| | `commands/raw_line.rs` (`RawLine` = Domänen-Wert) | `value_objects/`; danach `commands/` auflösen |
| | `handler/mod.rs` | `handler.rs` top-level |
| | `error.rs`, `ports/`, `value_objects/` (inkl. `ingestion_id`) | ✓; `models/`+`entities/` neu |
| `einlesen_adapter` | `bridges/` (calendar/money/source/store_bridge, ingested_publisher) | `adapter/` |
| | `commands/` (ingested_message, read_command) + `dto/` (line, raw_record) | in **einen** `models/`-Ordner |
| | `abstractions/`, `handler.rs`, `testapi/` | ✓ |
| `einlesen_specs` | `tests/einlesen.rs` mit inline-Fixtures (`gültiger_rohsatz`, `erwartete_zeile`, `feld`, `QUELLE`) | Fixtures → `fixtures/`; `tests/` nur nackte Tests; kein Stub |
| `obfuskierung_domain` | `commands/obfuscate_booking.rs` (`ObfuscateBooking`) | `models/`; `commands/` auflösen |
| | `handler/mod.rs` | `handler.rs` top-level |
| | `value_objects/ingested_booking.rs`, `value_objects/obfuscated_booking.rs` | `entities/` (Entscheidung 6) |
| | `value_objects/booking_id.rs` + übrige VOs, `ports/`, `error.rs` | ✓; `models/`+`entities/` neu |
| `obfuskierung_adapter` | `bridges/` (obfuscator/publisher/repository/source_bridge) | `adapter/` |
| | `commands/` (obfuscate_command, obfuscated_message) + `dto/` (ingested_row, obfuscated_booking) | in **einen** `models/`-Ordner |
| | `abstractions/`, `handler.rs`, `testapi/` | ✓ |
| `obfuskierung_specs` | `src/lib.rs`-Stub + `tests/obfuskierung.rs` mit inline-Fixtures (`gültige_zeile`, `api_mit_zeile`, `erwartete_buchung`, `einzige_buchung`, Consts) | Stub entfernen; Fixtures → `fixtures/`; `tests/` nur nackte Tests |
| `io` (`crates/io/`) | `einlesen.rs`, `obfuskierung.rs` flach | Ordner `einlesen/`, `obfuskierung/` (Crate bleibt `crates/io/`) |
| | `consumer.rs`, `dispatcher.rs`, `publishing.rs` flach | Ordner `common/` |
| `application_kernel` | `crates/application_kernel` (1 Trait/Datei, adapter-zugewandt) | ✓ unverändert — **kein** `kernel/`-Unterordner |
| `shared_kernel` | `crates/shared_kernel` (nur von `io` genutzt) | **nicht in 0031** — separates 0011-Issue löst es io-privat in `io` auf |

> `IngestedBooking` trägt sein `id: BookingId` sogar direkt als Feld — wie
> `ObfuscatedBooking` (Entscheidung 6) eine Entity, keine VO.

## Acceptance criteria

- [ ] **Domain-Crates:** Aggregate/Entities in `entities/`, VOs in `value_objects/`,
      Request/Config-Structs in `models/`; `error.rs` + `handler.rs` auf Top-Level; **kein**
      `aggregate/`-, `commands/`- oder `handler/`-Ordner mehr.
- [ ] **Adapter-Crates:** `adapter/` statt `bridges/`; **ein** zusammengeführter `models/`
      statt getrenntem `commands/` + `dto/`; `abstractions/`, `testapi/`, `handler.rs`
      unverändert.
- [ ] **Specs-Crates:** die spec-lokalen Fixtures liegen in `fixtures/`, die Testfiles
      enthalten nur **nackte** Tests; kein `src/lib.rs`-Stub; beide Crates identisch
      strukturiert.
- [ ] **`io`:** bleibt `crates/io/`; **intern** je Feature ein `einlesen/`- und
      `obfuskierung/`-Ordner, `consumer`/`dispatcher`/`publishing` in `common/`.
- [ ] **Kernel:** `application_kernel` bleibt `crates/application_kernel` unverändert (kein
      `kernel/`-Unterordner); `shared_kernel` unberührt (Auflösung nach `io` im separaten
      0011-Issue).
- [ ] Keine Crate wechselt den Ort — **keine** Workspace-`members`-Änderung, **keine**
      Berührung von `architecture`-Test oder Hook-Globs. Nur Verschiebungen, `mod`-/Pfad-
      und die eine Specs-`Cargo.toml`-Anpassung; keine Signatur-/Sichtbarkeits-/
      Verhaltensänderung an öffentlichem Code.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

None - can start immediately.
