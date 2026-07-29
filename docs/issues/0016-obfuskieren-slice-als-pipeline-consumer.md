---
id: "0016"
title: "obfuskieren-Slice als Pipeline-Consumer"
status: closed
---

# obfuskieren-Slice als Pipeline-Consumer

**Typ:** AFK

> **As-built — die Umsetzung weicht bewusst vom Wortlaut unten ab** (Nutzer-Entscheidungen
> in der Planungssitzung; maßgeblich sind **Code** + [Plan 0016-1](../plans/0016-1-obfuskieren-slice-pipeline-consumer.md)
> + [ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md)). Insbesondere:
> **seed-basiert statt First-Seen** (deterministisches Token aus `(Seed, Klartext)` via
> `Obfuscator`-Port, **kein** `ZuordnungsSpeicher`, **keine** Mapping-Tabelle); der Handler
> **liest das rohe Aggregat per ID** aus dem `einlesen`-Speicher (`RawBookingSource`,
> Message = Zeiger); der Guard (`assert_not_impl_any!`, kein `Serialize`) sitzt auf dem
> Egress-DTO `ObfuscatedBooking`; der Slice bindet **kein** `serde`. Der Textkörper unten
> ist die **ursprüngliche Issue-Fassung** (Vor-Bau-Stand) und bleibt als Kontext stehen.

> Neu aus [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md). Der
> `obfuskierung`-Slice (First-Seen-Aggregat `KONTO_x`/`PERSON_x`, laufstabil) wird zum
> **nachgelagerten Consumer** in der Pipeline umgebaut. **Datenschutz-Grenze:** ab
> hier laufen nur noch Token, roh **verlässt die Maschine nie**.

## Was zu bauen ist

Der zweite Pipeline-Schritt als Consumer-Slice: konsumiert die Outbox-Message
`Eingelesen` (aus [0015](0015-einlesen-slice-als-pipeline-consumer.md)), obfuskiert
und triggert den nächsten Schritt.

- **Handler** auf Domänen-Typen: treibt das First-Seen-Zuordnungs-Aggregat
  (`KONTO_x`/`PERSON_x`, laufstabil), vergibt Pseudonyme und scrubt Freitext
  ([ADR-0006](../adr/0006-pseudonym-stabilitaet.md)). Fail-safe „Firma vs.
  Privatperson": im Zweifel `Person` + Pseudonym.
- **Adapter**: Consume-Bridge (`Eingelesen` → Domänen-Command → Handler →
  `HandleOutcome`), Publisher-Bridge (→ Message `Obfuskiert`), Port-Bridges (der von
  der Domäne verlangte `ZuordnungsSpeicher` — Persistenz vertagt, In-Memory-Fake trägt
  ihn), Abstractions + TestApi/Fakes.
- Schreibt **Token-Tabelle** und die Message `Obfuskiert` in **einer** `UnitOfWork`.
- **Grenze:** die `assert_not_impl_any!`-„kein `Serialize`"-Sperre gilt ab hier für die
  Egress-DTOs (LLM/Report) — nicht für die lokale `einlesen`-Roh-Tabelle/-Message.

Die **laufübergreifende** Pseudonym-Stabilität (persistente Zuordnung,
[0008](0008-persistente-pseudonym-zuordnung.md)) und die vertieften **Scrubbing-Regeln**
([0010](0010-scrubbing-regeln-fuer-obfuskiertertext.md)) sind eigene Slices; dieser
Umbau nutzt vorerst die laufstabile First-Seen-Vergabe und die minimale
`ObfuskierterText`-Erstversion.

## Abnahmekriterien

- [ ] Der Slice konsumiert `Eingelesen`, vergibt `KONTO_x`/`PERSON_x` (First-Seen,
      laufstabil) und schreibt Token-Tabelle **und** Message `Obfuskiert` in **einer**
      `UnitOfWork` — über Fakes im Test belegt.
- [ ] Handler nutzt `shared_kernel::Handler` auf Domänen-Typen; die Consume-Bridge
      mappt `Response`/`Error` → `HandleOutcome`; kein public IO in den Domänen-Typen.
- [ ] Fail-safe-Auflösung „Firma vs. Privatperson" ist getestet (Unsicherheit →
      `Person` + Pseudonym).
- [ ] Ab diesem Slice tragen die Egress-relevanten Ausgangs-DTOs **keine** rohe
      IBAN/BIC/Kontonummer; der `assert_not_impl_any!`-Guard liegt auf dem Egress-DTO
      (nicht mehr auf `Iban`/`Bic`).
- [ ] `_specs` fahren den Slice als Black-Box über die Fluent-TestApi mit Fakes
      (Fake-`ZuordnungsSpeicher`, Fake-`OutboxPublisher`) — kein echtes IO.
- [ ] Der `architecture`-Test bleibt grün (Slice→Slice nur über injizierte
      `shared_kernel`-Interfaces / die Outbox-Message, keine `adapter→adapter`-Kante).
- [ ] `cargo build`/`test`/`clippy --all-targets` und `cargo fmt --check` grün.

## Blockiert durch

- Blockiert durch [0014](0014-shared-kernel-outbox-vertraege.md) (Outbox-Verträge).
- Blockiert durch [0015](0015-einlesen-slice-als-pipeline-consumer.md) (liefert die
  Message `Eingelesen`, die dieser Slice konsumiert).
