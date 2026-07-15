---
id: "0033"
title: "einlesen-Slice: oeffentliche Sichtbarkeit auf externe Flaeche verengen"
status: closed
---

# einlesen-Slice: oeffentliche Sichtbarkeit auf externe Flaeche verengen

**Typ:** AFK

## What to build

Reiner, mechanischer Sichtbarkeits-Umbau in der `einlesen`-Triade
(`einlesen_domain`, `einlesen_adapter`, `einlesen_specs`): Standard wird
**`pub(crate)`**; `pub` bleibt nur, was tatsächlich **über die Crate-Grenze
verarbeitet** wird. Heute ist praktisch jedes Struct/VO in der Domäne `pub` — das
kehren wir um: nach außen sichtbar bleibt nur die relevante Fläche, alles übrige ist
**by design intern**.

Nur Sichtbarkeit ändern — **keine** Logik, **keine** Signatur (außer `pub` →
`pub(crate)`), **keine** Datei-/Ordner-Struktur und **kein** öffentliches Verhalten.

Externe Fläche, die `pub` bleibt:

- **`einlesen_domain`:** nur der **Handler** (einziger Einstieg in die Domäne), die
  **Ports/Traits**, die der Adapter implementieren/mappen muss, und die zugehörigen
  **Transport-Models** (Request/Response, die die Grenze überqueren). Alle übrigen
  Typen — Value Objects, Entities/Aggregate, interne Helfer — werden `pub(crate)`.
- **`einlesen_adapter`:** nur die extern genutzte Fläche — die **abstractions**
  (public Seams/Traits) + **Transport-DTOs** (models) + **Handler** + **TestApi**.
  Interne Bridges/Adapter-Implementierungen und Helfer werden `pub(crate)`.
- **`einlesen_specs`:** Sichtbarkeit ebenso verengen — nichts unnötig `pub`
  (z. B. die Fixtures nur so sichtbar wie vom Test-Binary benötigt).

Was extern nachweislich gebraucht wird, gehört zur externen Fläche und bleibt `pub`
(der Compiler weist das aus); alles andere wird `pub(crate)`.

## Acceptance criteria

- [ ] In `einlesen_domain`, `einlesen_adapter` und `einlesen_specs` ist alles
      `pub(crate)`, außer der oben definierten externen Fläche
      (Domain: Handler + Ports/Traits + Transport-Models; Adapter: abstractions +
      models + Handler + TestApi; Specs: nichts unnötig `pub`).
- [ ] Kein Downstream-Crate verliert Zugriff auf etwas, das es real nutzt: wird ein
      Element nachweislich extern gebraucht, bleibt es `pub`.
- [ ] Keine Änderung an Logik, Signaturen (außer `pub` → `pub(crate)`),
      Datei-/Ordner-Struktur oder öffentlichem Verhalten.
- [ ] `cargo build`, `cargo clippy --all-targets`, `cargo test` und
      `cargo test -p architecture` sind grün.

## Blocked by

None - can start immediately.
