---
id: "0046"
title: Doku-Drift nach den Merges 0036-0045 auf den realen Stand ziehen
status: closed
---

# Doku-Drift nach den Merges 0036-0045 auf den realen Stand ziehen

**Typ:** AFK

## What to build

**Der Abschluss des ADR-0013-Baus** und der Nachfolger von
[0035](0035-architektur-doku-auf-realen-stand-ziehen.md), mit derselben Begründung: die
Architektur-Doku beschreibt sonst einen Stand, den es nicht mehr gibt, und jeder Agent, der danach
ein Ticket greift, liest sie **zuerst** und startet mit falschem Weltbild.

0035 hat den Drift einmal repariert — als **Prefactor vor** dem Bau. Dieses Ticket ist das
Gegenstück **nach** dem Bau: es zieht die Doku **einmal am Ende** auf den dann realen Stand, statt
sie über zehn Tickets hinweg zehnmal anzufassen. Genau darum steht es hinter
[0045](0045-io-auswerten-end-to-end.md), dem letzten Schnitt der Reihe.

Der Drift wird zu diesem Zeitpunkt die ganze Kette 0036–0045 umfassen. Was bereits heute
feststeht — aufgefallen bei der Ripple-Prüfung nach
[0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md), also erst knapp die Hälfte der
Strecke —, taugt als Startpunkt, **nicht** als abschließende Liste:

1. **Der Issue-Status lügt.** Mehrere Tickets stehen auf `blocked`, obwohl sie in `main` gemergt
   sind; nur eines wurde beim Merge korrekt geschlossen. Wer den Bestand nach offener Arbeit
   filtert, bekommt ein falsches Bild, und die `Blocked by`-Ketten der Folge-Tickets lesen sich als
   blockiert, obwohl sie frei sind.
2. **`PROGRESS.md` ist nicht mitgezogen** — dieselben Tickets stehen dort ungehakt, samt veralteter
   `Stand:`-Zeile.
3. **Die Crate-Liste in [CLAUDE.md](../../CLAUDE.md) nennt eine Zahl, die schon vor `auswerten`
   falsch war** (sie stammt aus der Zeit vor der `fenstern`-Triade). Ebenso der
   **Status-/Nächste-Schritte-Abschnitt**: er führt als gebaut nur `einlesen` und `obfuskieren` und
   kündigt als „Next" an, was bereits steht.
4. **Der Kopf von [ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md) sagt
   „noch nicht gebaut"**, obwohl der Bau läuft — nach 0045 ist er abgeschlossen.

Dazu kommt, was die restliche Kette absehbar veraltet: die Beschreibung des `auswerten`-Slice (beide
Reichweiten, `Fingerabdruck`, `Kategorisierer`-Orakel, Gegenkonto-Auflösung auf Token), die
Aufzählung der in `io` verdrahteten Slices, die Baureihenfolge und die Liste dessen, was noch
vertagt ist. Der Umsetzer arbeitet darum **nicht diese vier Punkte ab, sondern gleicht die Doku
gegen das Repo ab** — die Punkte oben sind die bekannten Startpunkte.

**Das Rustdoc der Crates gehört dazu** (nachgetragen bei der Ripple-Prüfung nach
[0043](0043-historie-vollstaendig-handler-und-fenster-uebergreifende-query.md)). Die ursprüngliche
Fassung dieses Tickets nannte nur CLAUDE.md, PROGRESS.md, README und die ADRs — womit die
Crate-Doku durch kein Auffangnetz gedeckt war, obwohl sie **dieselbe Sorte** Aussage trägt und ein
Agent sie **zuerst** liest, wenn er in einem Slice arbeitet. Konkret veraltet die Kette dort an
mindestens drei Stellen:

- die **Zustands-Aussagen zur `io`-Verdrahtung** je Slice (`auswerten_adapter` benennt heute
  explizit, dass `io` ihn noch nicht bindet) — [0045](0045-io-auswerten-end-to-end.md) macht sie
  falsch;
- das **„io erfüllt ihn"** der Seam- und Port-Docs, das erst ab 0045 bzw. dem realen `io` stimmt;
- die **„Noch nicht hier"**-Absätze der `lib.rs`-Köpfe, die auf Folge-Issues zeigen, die inzwischen
  gebaut sind.

Der Umsetzer prüft die Crate-Docs also **mit demselben Blick** wie die Projekt-Doku: behauptet ein
Satz einen Zustand, den es nicht (mehr) gibt?

**Reine Doku — kein Code-Verhalten ändert sich.** Bewusst **nicht** in diesem Ticket: eine Maßnahme
gegen die *Wiederholung* des Drifts (etwa das Schließen beim Merge verbindlich zu machen). Die
Ursache bleibt damit offen — aber sichtbar und absichtlich, statt in einem Reparatur-Ticket
mitzulaufen, dessen Fertig-Kriterium sie verwässern würde.

## Acceptance criteria

- [ ] Jedes Ticket der Kette, dessen Umsetzung in `main` liegt, trägt im Frontmatter
      `status: closed` — kein gemergtes Ticket steht mehr auf `blocked` oder `open`.
- [ ] `PROGRESS.md` spiegelt denselben Stand (Haken gesetzt, `Stand:`-Zeile aktuell), und die dort
      notierten Blockade-Hinweise widersprechen den Frontmatter-Status nicht mehr.
- [ ] Die in CLAUDE.md genannte Crate-Zahl und die aufgezählten Triaden stimmen mit dem überein, was
      real unter `crates/` liegt.
- [ ] Der Status-Abschnitt nennt als gebaut, was gebaut ist (inklusive der io-Verdrahtung je Slice),
      und als „Nächstes" das, was tatsächlich als Nächstes ansteht; die Liste des Vertagten enthält
      nichts mehr, was inzwischen steht.
- [ ] Die Beschreibung des `auswerten`-Slice nennt **beide** Reichweiten und die dazugehörigen
      Bausteine, statt ihn als ungebaut oder nur teilgebaut zu führen.
- [ ] Der Kopf von ADR-0013 behauptet nicht mehr, der Plan sei ungebaut.
- [ ] **Kein Satz** in CLAUDE.md, PROGRESS.md, README oder den ADRs widerspricht dem Repo — geprüft
      durch einen Abgleich gegen den realen Stand, nicht nur an den oben gelisteten Startpunkten.
- [ ] **Dasselbe gilt für das Rustdoc der Crates** (`lib.rs`-Köpfe, Seam-, Port- und Handler-Docs):
      keine Aussage über die `io`-Verdrahtung, über „von io erfüllte" Seams oder über „noch nicht
      Gebautes" steht mehr im Widerspruch zum Repo. Die Prüfung ist dieselbe Frage wie oben —
      behauptet ein Satz einen Zustand, den es nicht mehr gibt?
- [ ] Keine Code-Änderung; `cargo build`, `cargo test`, `cargo clippy --all-targets`,
      `cargo fmt --check` und `cargo test -p architecture` bleiben unverändert grün.

## Blocked by

Blocked by [0045](0045-io-auswerten-end-to-end.md) — die Doku wird **einmal am Ende** gezogen, wenn
die Kette vollständig ist; vorher stünde sie sofort wieder falsch.
