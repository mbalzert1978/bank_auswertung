---
id: "0026"
title: API-Guideline-Derives, must_use, Namen und Kosmetik
status: closed
---

# API-Guideline-Derives, must_use, Namen und Kosmetik

**Typ:** AFK

> Idiomatik-Review-Befund (Schwere: Niedrig, gebündelt). Kleinteilige Angleichungen an
> die Rust-API-Guidelines und den Code-in-Englisch-Grundsatz, die einzeln zu klein für
> ein eigenes Issue sind, aber zusammen einen sauberen Cleanup ergeben.

## Was zu bauen ist

Ein gebündelter Idiomatik-Cleanup ohne Verhaltensänderung:

- **`Debug`-Derive (C-DEBUG):** der öffentliche, re-exportierte Port-Ergebnis-Enum
  `BookingLookup` in `obfuskierung_domain` trägt heute keinerlei Derives, obwohl beide
  Nutzlasten `Debug` sind — Specs/Fakes können ihn dadurch nicht `{:?}`-formatieren oder
  in `assert_eq!` verwenden. `#[derive(Debug)]` (mindestens) ergänzen.
- **`#[must_use]` (C-MUST-USE):** die konsumierenden Builder der TestApis
  (`new`/`with_*`, die `Self` per Value zurückgeben) und die seiteneffektfreien
  Value-Object-Konstruktoren/Getter der Domäne — ein weggeworfenes Ergebnis ist dort
  stets ein Bug. `#[must_use]` dort ergänzen, wo es Absicht absichert statt Rauschen zu
  erzeugen.
- **Code in Englisch:** die `From<NonEmptyString>`-Impls in `einlesen_domain` benennen
  den Parameter deutsch (`wert`), während der Rest englische Bezeichner nutzt (`value`).
  Auf `value` vereinheitlichen.
- **Kleine Kosmetik:** der `match` auf `split_once` im `FakeMoneySeam` wird
  `.split_once(',').unwrap_or(...)`; der unnötige Deref `(*id).to_string()` in der
  `obfuskierung`-TestApi wird `ToString::to_string`; das leere `pub mod`-Doku-Anker-Modul
  in `shared_kernel` wird durch einen `#[doc]`-Kommentar an einem realen Item ersetzt
  (statt ein leeres öffentliches Modul zu exportieren).

## Abnahmekriterien

- [ ] `BookingLookup` ist `Debug` und in `assert_eq!`/`{:?}` verwendbar.
- [ ] Konsumierende TestApi-Builder und seiteneffektfreie Domänen-Konstruktoren/Getter
      tragen `#[must_use]`, wo es die Verwendungsabsicht absichert.
- [ ] Die `From<NonEmptyString>`-Parameter in `einlesen_domain` heißen `value`; kein
      deutscher Bezeichner mehr im Code dieser Impls.
- [ ] `FakeMoneySeam`-`split_once` als Kombinator, der unnötige `(*id)`-Deref entfernt,
      der leere `pub mod`-Doku-Anker durch `#[doc]` ersetzt.
- [ ] Reiner Cleanup ohne Verhaltensänderung: alle Slice-Specs bleiben grün;
      `cargo build`/`test`/`clippy --all-targets`/`fmt --check` und
      `cargo test -p architecture` grün; keine neuen externen Dependency-Einträge.

## Blockiert durch

- None - can start immediately.

**Reihenfolge (kein Blocker, aber Datei-Überlappung):** dieser Cleanup streut über vier
Crates und fasst dabei `obfuskierung`-Adapter/-Domäne-Dateien an, die
[0024](0024-dto-domaene-mapping-auf-from-tryfrom-umstellen-tryfrom-wenn-fallibel-sonst-from.md)
und [0025](0025-vermeidbare-clones-auf-fehlerpfaden-entfernen.md) zuvor umbauen. Im
`obfuskierung`-Strang **zuletzt** fahren: **0024 → 0025 → 0026 seriell**, damit die Kosmetik
auf den bereits gesetzten Dateien landet. Parallel dazu sind
[0023](0023-error-ketten-vervollstaendigen-source-an-wrapper-errors.md) und
[0027](0027-spec-testcode-aufraeumen-let-else-vec-wiederholung-fixture-helfer.md) disjunkt.
