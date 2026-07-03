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
| [0006](0006-pseudonym-stabilitaet.md) | Pseudonym-Stabilität | in Teilen superseded | [0007](0007-modularer-monolith-feature-slices.md) |
| [0007](0007-modularer-monolith-feature-slices.md) | Modularer Monolith aus Feature-Slices | in Teilen superseded | [0008](0008-event-getriebene-pipeline-und-outbox.md) |
| [0008](0008-event-getriebene-pipeline-und-outbox.md) | Event-getriebene Pipeline aus Slices mit Outbox | **maßgeblich** | — |

## Supersession-Graph (Kante zeigt vom Ablöser zum abgelösten ADR)

```
0008 ──▶ 0001, 0002, 0003, 0004, 0007   (supersedet/verfeinert; kanonischer Abschnitt in 0008)
0007 ──▶ 0001, 0002, 0003, 0004, 0005, 0006   (Architektur-/Modell-Verortung)
```

- **[ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) ist maßgeblich** — die
  event-getriebene Pipeline aus Slices mit Outbox. Es supersedet/verfeinert 0001,
  0003, 0004, 0007 und präzisiert 0002 (siehe dort „Was dieses ADR supersedet").
- **[ADR-0007](0007-modularer-monolith-feature-slices.md)** führte den modularen
  Monolithen aus Feature-Slice-Triaden ein und verfeinerte damit die
  Architektur-/Modell-Aussagen von 0001–0006; es ist seinerseits von 0008 in Teilen
  superseded.
- Der **Textkörper** eines abgelösten ADR ist eingefrorene Historie und **nicht
  implementierungsmaßgeblich**; er bleibt als Entscheidungskontext erhalten.
