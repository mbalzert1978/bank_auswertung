# Architecture Decision Records — Übersicht & Supersession-Graph

Diese Seite ist ein **reiner Index**: Status und „wer supersedet wen" über alle
ADRs. Sie kopiert **keinen** Regelinhalt — was heute gilt, steht jeweils im
verlinkten ADR selbst (kanonisch für die abgelösten ADRs im Abschnitt „Was dieses
ADR supersedet" von [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)).

## ADRs

| ADR | Titel | Status | Superseded / verfeinert durch |
|-----|-------|--------|-------------------------------|
| [0001](0001-hexagonale-architektur.md) | Hexagonale Architektur | in Teilen superseded | [0007](0007-modularer-monolith-feature-slices.md), [0008](0008-event-getriebene-pipeline-und-outbox.md) |
| [0002](0002-dto-grenze-und-anti-corruption-layer.md) | DTO-Grenze & Anti-Corruption-Layer | in Teilen superseded | [0007](0007-modularer-monolith-feature-slices.md), [0008](0008-event-getriebene-pipeline-und-outbox.md) |
| [0003](0003-rust-als-sprache.md) | Rust als Sprache | in Teilen superseded | [0007](0007-modularer-monolith-feature-slices.md), [0008](0008-event-getriebene-pipeline-und-outbox.md) |
| [0004](0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md) | Domäne ist die Analyse, LLM nur Orakel | in Teilen superseded | [0007](0007-modularer-monolith-feature-slices.md), [0008](0008-event-getriebene-pipeline-und-outbox.md) |
| [0005](0005-domaenenmodell-value-objects-und-summentypen.md) | Domänenmodell: Value Objects & Summentypen | in Teilen superseded | [0007](0007-modularer-monolith-feature-slices.md) |
| [0006](0006-pseudonym-stabilitaet.md) | Pseudonym-Stabilität | superseded | [0010](0010-seed-basierte-pseudonym-stabilitaet.md) (Mechanismus), [0007](0007-modularer-monolith-feature-slices.md) (Verortung) |
| [0007](0007-modularer-monolith-feature-slices.md) | Modularer Monolith aus Feature-Slices | in Teilen superseded | [0008](0008-event-getriebene-pipeline-und-outbox.md), [0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md) |
| [0008](0008-event-getriebene-pipeline-und-outbox.md) | Event-getriebene Pipeline aus Slices mit Outbox | **maßgeblich** (in Teilen verfeinert) | [0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md) (Kernel-Aspekt), [0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) (Slice-Mechanik), [0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) (Fehler-Taktik + ID-Handoff) |
| [0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md) | Zwei Kernel-Crates + idiomatische Outbox-Verträge | **maßgeblich** (in Teilen verfeinert) | [0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) (Kernel-Flip + Fehler-Mapping) |
| [0010](0010-seed-basierte-pseudonym-stabilitaet.md) | Seed-basierte Pseudonym-Stabilität | **maßgeblich** | — |
| [0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md) | Slice-reine Pipeline, Outbox-Mechanik in `io` | **maßgeblich** (entschiedener Plan; in Teilen superseded) | [0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) (Entscheidung B — Fehler-Taktik) |
| [0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) | Durchweg-terminale Pipeline + uniformes ID-Handoff | **maßgeblich** | — |

## Supersession-Graph (Kante zeigt vom Ablöser zum abgelösten ADR)

```
0012 ──▶ 0008, 0011   (durchweg-terminale Fehler-Taktik + uniformes ID-Handoff; kanonisch in 0012)
0011 ──▶ 0008, 0009   (Slice = reine Pipeline, Outbox-Mechanik + Kernel-Flip in io; kanonisch in 0011)
0010 ──▶ 0006   (Pseudonym-Stabilität: Seed statt persistenter Zuordnungstabelle; kanonisch in 0010)
0009 ──▶ 0007, 0008   (Kernel-Split + idiomatische Outbox-Verträge; kanonisch in 0009)
0008 ──▶ 0001, 0002, 0003, 0004, 0007   (supersedet/verfeinert; kanonischer Abschnitt in 0008)
0007 ──▶ 0001, 0002, 0003, 0004, 0005, 0006   (Architektur-/Modell-Verortung)
```

- **[ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) ist maßgeblich** — die
  event-getriebene Pipeline aus Slices mit Outbox. Es supersedet/verfeinert 0001,
  0003, 0004, 0007 und präzisiert 0002 (siehe dort „Was dieses ADR supersedet"). Seine
  **Pipeline-Garantien** bleiben maßgeblich; 0011 verfeinert die **Slice-Mechanik-Verortung**
  und 0012 die **Fehler-Taktik + das ID-Handoff** (siehe dort).
- **[ADR-0012](0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) ist maßgeblich**
  — die Pipeline ist **durchweg terminal** (kein Dead-Letter/Retry im Slice; alle drei
  I/O-Schritte read/save/publish fehlbar), und das **ID-Handoff ist uniform**: die von der
  Ablage **neu vergebene** ID ist der publizierte Zeiger, Aggregate referenzieren
  Fremd-Entitäten **nur per ID**. Es korrigiert die „Dead-Letter"/`io`-Klassifikations-
  Erzählung von 0011 (Entscheidung B) als Missverständnis und präzisiert die
  Envelope-/Handoff-Aussagen von 0008.
- **[ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md)** ist der **entschiedene
  Plan**, jeden Slice zur reinen `Command → Domäne → Response`-Pipeline zu entkoppeln: die
  Consumer-Rolle, `UnitOfWork` und Envelope-Kenntnis wandern nach `io` (generische
  `OutboxConsumer<H>`), der Kernel-Schnitt aus 0009 flippt (`application_kernel`
  `io`-sichtbar, `shared_kernel` `io`-privat). Entscheidung A/C/D bleiben maßgeblich;
  Entscheidung B (Fehler-Taktik) ist durch 0012 superseded.
- **[ADR-0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md) verfeinert den
  Kernel-Aspekt** von 0008 (und 0007): der eine `shared_kernel` wird in `shared_kernel`
  (io-facing Outbox-Seams) + `application_kernel` (slice-interne Handler-Konventionen)
  geschnitten — Kapselung über den Crate-Schnitt, weil Rust kein cross-crate
  `internal` kennt — und die Outbox-Verträge werden Rust-idiomatisch ausformuliert
  (`Result`/`OutboxError`, `publish`, `const MESSAGE_TYPE`, `TypedOutboxHandler`). Die
  Pipeline-Aussagen von 0008 bleiben maßgeblich.
- **[ADR-0010](0010-seed-basierte-pseudonym-stabilitaet.md)** löst den
  Stabilitäts-**Mechanismus** von 0006 ab: die Pseudonym-Stabilität ruht auf einem
  **config-injizierten Seed** (deterministische Token via `Obfuscator`-Port,
  mitpersistiert), **nicht** auf einer persistenten Klartext↔Pseudonym-Zuordnung; die
  Datenschutz-Grenze ist die **Maschinen-Grenze** (LLM/Report), Beleg auf dem Egress-DTO.
- **[ADR-0007](0007-modularer-monolith-feature-slices.md)** führte den modularen
  Monolithen aus Feature-Slice-Triaden ein und verfeinerte damit die
  Architektur-/Modell-Aussagen von 0001–0006; es ist seinerseits von 0008 in Teilen
  superseded.
- Der **Textkörper** eines abgelösten ADR ist eingefrorene Historie und **nicht
  implementierungsmaßgeblich**; er bleibt als Entscheidungskontext erhalten.
