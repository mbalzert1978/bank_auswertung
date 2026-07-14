---
id: "0024"
title: DTO↔Domäne-Mapping auf From/TryFrom umstellen (TryFrom wenn fallibel, sonst From)
status: open
---

# DTO↔Domäne-Mapping auf From/TryFrom umstellen (TryFrom wenn fallibel, sonst From)

**Typ:** AFK

> Idiomatik-Review-Befund (Schwere: Mittel, wiederkehrend über mehrere Crates). Die
> ACL-Übersetzungen zwischen Adapter-DTOs und Domäne stehen heute als Inline-`match`
> in Konstruktoren bzw. als benannte `from_domain`/`from_aggregate`/`to_*`-Methoden.
> **Entschiedene Konvention:** die Standard-Konversions-Traits verwenden —
> **`TryFrom` wenn die Abbildung fehlschlagen kann, sonst `From`.**

## Was zu bauen ist

Die **ein-eingabigen** Summentyp- und Projektions-Abbildungen an der DTO↔Domäne-Grenze
werden auf die Standard-Konversions-Traits gehoben, damit der Grenzübertritt im
Standard-Vokabular (`.into()` / `.try_into()`) sichtbar wird und die Mapping-Regel an genau
der dafür idiomatischen Stelle lebt (Rust-API-Guideline C-CONV). Die Regel gilt für jede
Abbildung mit **genau einer** Eingabe: **fallible → `TryFrom` (mit dem bisherigen Fehlertyp
als `Error`), infallible → `From`.**

**Ausdrücklich außerhalb des Umfangs:** mehr-eingabige Konversionen bleiben benannte
Methoden — `From`/`TryFrom` sind ein-eingabig und können ein Extra-Argument nicht tragen.
Konkret bleibt `obfuskierung_adapter`s `IngestedRow::to_booking(&self, id: BookingId)`
(Zeile + separate `BookingId`) eine benannte Methode; ihre Ownership wird getrennt in
[Issue 0025](0025-vermeidbare-clones-auf-fehlerpfaden-entfernen.md) gestrafft, **nicht
hier**.

Betroffen sind — repo-weit konsistent, damit `auswerten`/`io` später denselben Weg
erben — die folgenden Kategorien (Pfade illustrativ, nicht bindend):

- **`einlesen_adapter`:** die infalliblen Summentyp-Übersetzungen (`Currency`,
  `Direction`, `Sign` — heute Inline-`match` in `Line`/`money_bridge`) werden `From`; die
  infallible Projektion `Line::from_aggregate` wird `From<&LineAggregate>`; die **fallible**
  `RawRecord::to_raw_line` wird `TryFrom<&RawRecord>` (Fehlertyp `MissingMandatoryField`
  bleibt sichtbar wie heute, `pub(crate)`).
- **`obfuskierung_adapter`:** die benannten `from_domain`/`from_token`-Projektionen der
  Egress-/Ingress-DTOs werden `From`-Impls (infallibel).
- **`io`:** die Inline-Übersetzung `TransactionError<E> → OutboxError` in der
  `OutboxConsumer`-Mechanik wird `From`, die Aufrufstelle nutzt `map_err(OutboxError::from)`.

Die Sichtbarkeit bleibt (`pub(crate)` wo heute so); es wird **kein Domänen-Typ
re-exportiert** — die adapter-eigenen DTO-Summentypen bleiben adapter-owned (bewusste
Re-Deklaration, kein Finding). Der Compile-Zwang bei einer neuen Domänen-Variante bleibt
durch das exhaustive `match` im Trait-Body identisch.

## Abnahmekriterien

- [ ] Alle genannten **ein-eingabigen** DTO↔Domäne-Abbildungen sind `From`- bzw.
      `TryFrom`-Impls; die Regel „fallibel → `TryFrom`, sonst `From`" ist auf sie angewandt
      (u. a. `to_raw_line` → `TryFrom`, alle Summentyp- und Projektions-Mappings → `From`).
      `IngestedRow::to_booking` (mehr-eingabig) bleibt unverändert eine benannte Methode.
- [ ] Die Aufrufstellen nutzen `.into()`/`.try_into()`/`map_err(_::from)` statt der
      bisherigen benannten Methoden; die entfernten `from_*`/`to_*`-Methoden hinterlassen
      keine toten Aufrufer.
- [ ] Kein Domänen-Typ wird über die Adapter-Grenze re-exportiert; die adapter-eigenen
      DTO-Typen bleiben erhalten; `cargo test -p architecture` grün.
- [ ] `einlesen`- und `obfuskierung`-Slice-Specs bleiben Black-Box über die TestApi und
      unverändert grün; `cargo build`/`test`/`clippy --all-targets`/`fmt --check` grün.

## Blockiert durch

- None - can start immediately (die Konvention „TryFrom wenn fallibel, sonst From" ist
  bereits entschieden).

**Reihenfolge (kein Blocker, aber Datei-Überlappung):** dieses Issue reshaped die
`obfuskierung_adapter`-Mappings, die [0025](0025-vermeidbare-clones-auf-fehlerpfaden-entfernen.md)
(Ownership) und [0026](0026-api-guideline-derives-must-use-namen-und-kosmetik.md) (Kosmetik)
danach anfassen. Der `obfuskierung`-Strang **0024 → 0025 → 0026 seriell** fahren (nicht
parallel), sonst kollidieren die PRs auf denselben Dateien. Parallel zu diesem Strang sind
[0023](0023-error-ketten-vervollstaendigen-source-an-wrapper-errors.md) und
[0027](0027-spec-testcode-aufraeumen-let-else-vec-wiederholung-fixture-helfer.md) disjunkt.
