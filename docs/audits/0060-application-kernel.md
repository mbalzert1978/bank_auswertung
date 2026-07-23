---
id: "0060-audit"
title: "Prüf-Report: Gegenprüfung `application_kernel` gegen die `.rules`"
status: abgeschlossen
---

# Prüf-Report — `application_kernel` gegen die `.rules`

> Bezug: Issue [0060](../issues/0060-gegenpruefung-application-kernel-rules.md)
> (GitHub [#59](https://github.com/mbalzert1978/bank_auswertung/issues/59)).
> **Reines Audit — kein Produktivcode geändert.** Dieser Report ist der einzige
> Datei-Zusatz des zugehörigen PR.

## Umfang & Methode

Geprüft wurde ausschließlich der Code der Crate `crates/application_kernel` gegen die im
Ticket gelisteten einschlägigen `.rules`-Regeln. Der Kernel ist ein **interface-only
Vertrag** (fünf Trait-Dateien + `lib.rs` + `Cargo.toml`, kein I/O, keine DB, keine
Domänen-Rechnung), daher sind nur die im Ticket benannten Regeln anwendbar; die
Domänenmodell-, Testkultur-, SQL-, Nebenläufigkeits- und Performance-Regeln greifen für eine
reine Trait-Crate nicht.

Geprüfte Dateien:

- `crates/application_kernel/Cargo.toml`
- `crates/application_kernel/src/lib.rs`
- `crates/application_kernel/src/command.rs`
- `crates/application_kernel/src/command_decoder.rs`
- `crates/application_kernel/src/handler.rs`
- `crates/application_kernel/src/publish_command.rs`
- `crates/application_kernel/src/publisher.rs`

Verifikationsschritte: Sichtung aller Quelldateien, `grep` nach externen `use`/`extern
crate`, Auflösung der Crate-Dependencies über `cargo metadata --no-deps` und Sichtung der
`[lints]`- bzw. `[workspace.lints]`-Konfiguration.

Hinweis zur Redaktion: Der Kernel enthält als reine Trait-Crate keinerlei rohe
Identifikatoren, IBANs, Klartext-Namen oder Seeds; es gab folglich nichts zu redigieren.

## Befunde je Regel

| # | Regel | Befund | Beleg |
|---|-------|--------|-------|
| 1 | `.rules/crates/dependencies-und-kapselung.md` — Kernel dependency-frei, einziger Slice↔`io`-Vertrag | **konform** | `Cargo.toml:9` (`[dependencies]` leer); `cargo metadata --no-deps` ⇒ `application_kernel -> []`; keine externen `use`, nur `use crate::…` (`handler.rs:1`, `publisher.rs:1`, `publish_command.rs:1`) |
| 2 | `.rules/rust/idiome.md` — interface-only, kein externes Crate | **konform** | Alle fünf Quelldateien definieren ausschließlich Traits; einzige Fremd-Bindung ist `std::error::Error` (`handler.rs:23`) aus der Standardbibliothek — kein `serde`/`thiserror`/`static_assertions` |
| 3 | `.rules/common/architektur.md` — Kontrakte tragen die Outbox-Kopplung als **Vertrag**, keine Slice-/`io`-internen Typen sickern durch | **konform (mit einer Doku-Anmerkung, s. u.)** | `CommandDecoder::decode` nimmt nur nackte `&str` (`command_decoder.rs:20`, „Shape Y"); `PublishCommand` trägt nur Dispatch-Key + `partition_key` (`publish_command.rs:17,23`); keine `Outbox*`-/Slice-Typen in einer Signatur |
| 4 | `.rules/common/sprache.md` — Doc-Kommentare Deutsch, Mechanik-Bezeichner Englisch | **konform** | Sämtliche `//!`/`///`-Kommentare deutsch; alle Bezeichner (`Command`/`Handler`/`Publisher`/`PublishCommand`/`CommandDecoder`/`decode`/`handle`/`publish`/`partition_key`/`MESSAGE_TYPE`) englisch — korrekt, da Infrastruktur, keine Domänen-Fachbegriffe |
| 5 | `.rules/rust/gates.md` — `[lints] workspace = true` gesetzt | **konform** | `Cargo.toml:6-7`; das Workspace-Root definiert die geerbten Sets (`[workspace.lints.clippy]`, `[workspace.lints.rustdoc]`) |

## Akzeptanzkriterien

- **Jede gelistete Regel geprüft, je Regel ein Befund mit `file:line`** — erfüllt (Tabelle oben).
- **`Cargo.toml` ohne Fremd-Dependency; `[lints] workspace = true` bestätigt** — erfüllt
  (`Cargo.toml:6-9`, `cargo metadata` ⇒ leere Dependency-Liste).
- **Keine Slice-/`io`-internen Typen im Kernel** — bestätigt: auf der **Typ-Oberfläche**
  (Signaturen, assoziierte Typen, Konstanten) taucht kein `io`-privater Outbox-Typ und kein
  Slice-Typ auf. Die einzigen Erwähnungen von `OutboxCommand`/`OutboxMessage`/`OutboxHandler`
  stehen als **Prosa in Doc-Kommentaren** (z. B. `publish_command.rs:5`, `lib.rs:11`) — eine
  Namensnennung, kein Typ-Bezug; das ist nach `.rules/rust/gates.md` ausdrücklich erlaubt,
  solange es kein toter Doc-Link ist (hier sind es Code-Spans ohne Link).
- **Ergebnis als redigierter Report abgelegt, Abweichungen als Folge-Ticket** — dieser Report;
  Vorschläge unten. **Kein Code geändert.**

## Anmerkung (Doku-Drift, keine harte Regelabweichung)

Der Vertrag selbst ist sauber; dennoch trägt **ein Doc-Kommentar** eine überholte Erzählung:

- `command_decoder.rs:12-13` und `:19-20` beschreiben, dass `io` ein `None` aus `decode`
  „auf ein `Critical`-Dead-Letter" abbildet.

Diese „`io` klassifiziert Warning/Retry/Critical / Dead-Letter"-Erzählung ist laut
`CLAUDE.md` und [ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md)
ausdrücklich **ein Missverständnis**: Die Pipeline ist **durchweg terminal**, es gibt **kein**
Dead-Letter/Retry, und **wie** `io` einen terminalen Fehlschlag surfacet, ist **vertagt**.
Das Doc-Kommentar nimmt also eine bereits superseded Mechanik als gesetzt an. Das ist keine
Verletzung der fünf Ticket-Regeln auf der **Vertrags-/Typ-Ebene** (dort ist alles konform)
und auch kein Verstoß gegen „kein Dead-Letter **im Slice**" (der Kernel ist kein Slice) —
aber es ist eine dokumentarische Drift, die einem Leser eine falsche `io`-Semantik suggeriert.
Behebung ist eine **reine Doc-Kommentar-Anpassung** und gehört bewusst **nicht** in dieses
Audit-Ticket (kein Code ändern), sondern in ein Folge-Ticket.

## Folge-Ticket-Vorschläge

1. **Titel:** „`CommandDecoder`-Doc-Kommentar auf ADR-0012 (durchweg terminal) ziehen —
   `Critical`-Dead-Letter-Narrativ entfernen"
   **Betroffene Regel/Quelle:** `.rules/common/architektur.md` (Abschnitt „Durchweg terminal
   — kein Retry, kein Dead-Letter"), [ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md).
   **Ort:** `crates/application_kernel/src/command_decoder.rs:12-13`, `:19-20`.
   **Art:** reine Doku-Korrektur (Doc-Kommentar), kein Verhaltenswechsel. `None` als „leerer/
   ungültiger Inhalt" beschreiben, ohne die vertagte `io`-Fehler-Surfacing-Mechanik als
   `Critical`-Dead-Letter vorwegzunehmen.

## Fazit

Fünf von fünf einschlägigen Regeln **konform**. Keine harte Regelabweichung auf der
Vertrags-/Typ-Ebene: Der Kernel ist dependency-frei, interface-only, deutschsprachig
dokumentiert mit englischen Mechanik-Bezeichnern, und `[lints] workspace = true` ist gesetzt.
Es bleibt **eine** dokumentarische Drift (überholtes `Critical`-Dead-Letter-Narrativ in einem
Doc-Kommentar), die als **ein** Folge-Ticket geschnitten ist — als reine Doku-Korrektur außerhalb
dieses Audits.
