---
id: "0023"
title: Error-Ketten vervollständigen — source() an Wrapper-Errors
status: closed
---

# Error-Ketten vervollständigen — source() an Wrapper-Errors

**Typ:** AFK

> Idiomatik-Review-Befund (Schwere: Mittel). Die `_domain`-Crates schreiben ihre
> Fehler bewusst von Hand (kein `thiserror`, [ADR-0005](../adr/0005-domaenenmodell-value-objects-und-summentypen.md)),
> dabei bleibt an einem Wrapper-Enum die Ursachenkette liegen.

## Was zu bauen ist

Ein hand-geschriebener Wrapper-Error, der über zwei bereits `std::error::Error`
implementierende Fehler summiert, muss deren Ursache über `source()` weiterreichen —
sonst ist die Kette gebrochen und ein Konsument (Logging, `anyhow`-Chain) sieht die
innere Ursache nie. Konkret betrifft das `einlesen_domain`s `ReadError` (Varianten
`Source(SourceError)` / `Line(LineError)`), dessen `impl std::error::Error` heute leer
ist; die `Display`-Delegation ersetzt `source()` nicht (Rust-API-Guideline C-GOOD-ERR).

Das Issue ergänzt `source()` so, dass es je Variante den inneren Fehler zurückgibt,
und prüft die übrigen hand-geschriebenen Wrapper-Errors der Domänen-Crates auf denselben
Befund. Der Prüf-Umfang ist geschlossen und klein: der einzige weitere Kandidat mit
innerer Ursache ist `obfuskierung_domain`s `HandleError` — hat er eine gewrappte Ursache,
liefert `source()` sie, sonst bleibt sein `impl` leer. Blatt-Errors ohne innere Ursache
(`LineError`, `SourceError`, `MissingMandatoryField`) bleiben korrekt mit leerem `impl`.
Optional, weil es den Handler entzerrt: `From`-Impls für die gewrappten Fehler, damit der
Handler `?` statt `.map_err(ReadError::Source)?` nutzen kann.

**Redaktions-Invariante bleibt gewahrt:** `source()` legt nur den inneren Fehler offen,
dessen `Display` bereits redigiert ist (Feldname statt Rohwert) — es entsteht kein neuer
Leak-Pfad. Diese Invariante ist testbar und muss grün bleiben.

## Abnahmekriterien

- [ ] `ReadError::source()` liefert für jede Variante den inneren Fehler
      (`Source`/`Line`), statt `None`.
- [ ] `obfuskierung_domain`s `HandleError` ist auf die gebrochene Kette geprüft; hat er
      eine innere Ursache, liefert `source()` sie, sonst bleibt sein `impl` leer.
      Blatt-Errors (`LineError`, `SourceError`, `MissingMandatoryField`) bleiben mit
      leerem `impl` korrekt.
- [ ] Die batch-atomare Redaktions-Invariante (Dead-Letter-Grund spiegelt nie einen
      Rohwert) bleibt über die bestehenden Slice-Specs grün.
- [ ] `cargo build`/`test`/`clippy --all-targets`/`fmt --check` und
      `cargo test -p architecture` grün; kein neuer externer Dependency-Eintrag in einer
      `_domain`-Crate.

## Blockiert durch

- None - can start immediately.
