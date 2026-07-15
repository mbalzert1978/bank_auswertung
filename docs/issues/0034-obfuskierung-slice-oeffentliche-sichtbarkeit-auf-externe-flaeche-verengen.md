---
id: "0034"
title: "obfuskierung-Slice: oeffentliche Sichtbarkeit auf externe Flaeche verengen"
status: closed
---

# obfuskierung-Slice: oeffentliche Sichtbarkeit auf externe Flaeche verengen

**Typ:** AFK

## What to build

Reiner, mechanischer Sichtbarkeits-Umbau in der `obfuskierung`-Triade
(`obfuskierung_domain`, `obfuskierung_adapter`, `obfuskierung_specs`): Standard wird
**`pub(crate)`**; `pub` bleibt nur, was tatsächlich **über die Crate-Grenze
verarbeitet** wird. Heute ist praktisch jedes Struct/VO in der Domäne `pub` — das
kehren wir um: nach außen sichtbar bleibt nur die relevante Fläche, alles übrige ist
**by design intern**.

Nur Sichtbarkeit ändern — **keine** Logik, **keine** Signatur (außer `pub` →
`pub(crate)`), **keine** Datei-/Ordner-Struktur und **kein** öffentliches Verhalten.
Insbesondere bleibt der `assert_not_impl_any!`-No-`Serialize`-Guard auf dem
Egress-Projektions-DTO unangetastet.

Externe Fläche, die `pub` bleibt:

- **`obfuskierung_domain`:** nur der **Handler** (einziger Einstieg in die Domäne),
  die **Ports/Traits**, die der Adapter implementieren/mappen muss, und die
  zugehörigen **Transport-Models** (Request/Response, die die Grenze überqueren).
  Alle übrigen Typen — Value Objects, Entities/Aggregate, interne Helfer — werden
  `pub(crate)`.
- **`obfuskierung_adapter`:** nur die extern genutzte Fläche — die **abstractions**
  (public Seams/Traits) + **Transport-DTOs** (models, inkl. der Egress-Projektion)
  + **Handler** + **TestApi**. Interne Bridges/Adapter-Implementierungen und Helfer
  werden `pub(crate)`.
- **`obfuskierung_specs`:** Sichtbarkeit ebenso verengen — nichts unnötig `pub`
  (z. B. die Fixtures nur so sichtbar wie vom Test-Binary benötigt).

Was extern nachweislich gebraucht wird, gehört zur externen Fläche und bleibt `pub`
(der Compiler weist das aus); alles andere wird `pub(crate)`.

## Acceptance criteria

- [ ] In `obfuskierung_domain`, `obfuskierung_adapter` und `obfuskierung_specs` ist
      alles `pub(crate)`, außer der oben definierten externen Fläche
      (Domain: Handler + Ports/Traits + Transport-Models; Adapter: abstractions +
      models + Handler + TestApi; Specs: nichts unnötig `pub`).
- [ ] Kein Downstream-Crate verliert Zugriff auf etwas, das es real nutzt: wird ein
      Element nachweislich extern gebraucht, bleibt es `pub`.
- [ ] Keine Änderung an Logik, Signaturen (außer `pub` → `pub(crate)`),
      Datei-/Ordner-Struktur oder öffentlichem Verhalten; der No-`Serialize`-Guard auf
      dem Egress-DTO bleibt bestehen.
- [ ] `cargo build`, `cargo clippy --all-targets`, `cargo test` und
      `cargo test -p architecture` sind grün.

## Blocked by

None - can start immediately. Unabhängig von
[0033](0033-einlesen-slice-oeffentliche-sichtbarkeit-auf-externe-flaeche-verengen.md).
