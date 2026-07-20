---
id: "0048"
title: "rustdoc-Warnungen: public Doku zeigt auf private Items"
status: open
---

# rustdoc-Warnungen: public Doku zeigt auf private Items

**Typ:** AFK

## What to build

`cargo doc --no-deps --workspace` meldet **24 Warnungen** der Form „public documentation for `X`
links to private item `Y`". Sie verteilen sich über **alle vier Adapter-Crates** und **zwei
Domänen-Crates** — es ist kein Ausrutscher eines Slice, sondern ein Muster, das seit `einlesen`
unbemerkt mitgewachsen ist, weil nichts es prüft.

Die Warnungen sind nicht kosmetisch: ein Link auf ein privates Item wird im generierten Doc **gar
nicht** gesetzt. Der Leser der öffentlichen Fläche sieht einen Verweis, der ins Leere zeigt — und
genau die Erklärung, die ihn zum Kollaborateur führen sollte, fehlt.

**Drei Ursachen**, die getrennt zu behandeln sind:

1. **Public Seam-Doku verlinkt ihre private Port-Bridge** (der Löwenanteil). Jede `*Seam`-Abstraction
   erklärt sich über die Bridge, die sie auf den Domänen-Port mappt — die Bridges liegen aber
   **bewusst privat** (ADR-0011: die Verdrahtung ist Slice-intern, nur der Seam ist öffentlich).
2. **Crate-Doku verlinkt die privaten Module** `adapter` bzw. `abstractions`.
3. **Public Doku verlinkt private Methoden/Konstanten** — eine private Konstruktor-Methode, ein
   Fortsetzungs-Helfer, eine Ketten-Schranke der TestApi.

**Der Fix liegt auf der Doku-Seite, nicht an der Sichtbarkeit.** Die Bridges public zu machen wäre
der falsche Weg: das weichte die Schichtung auf, die der `architecture`-Test bewacht, nur damit ein
Doc-Link auflöst. Die Prosa soll den Kollaborateur weiterhin **benennen** dürfen — sie soll ihn nur
nicht als Link ausgeben, der keiner werden kann.

Dazu die **Regressionssperre**: ohne sie wächst das Muster weiter, wie es das seit `einlesen` getan
hat. Der Lint für private Intra-Doc-Links gehört in die Workspace-Lints, damit ein neuer Verstoß
**beim Bauen** auffällt statt zufällig im nächsten Review — und `cargo doc` wird zum sechsten Gate
neben `build`/`test`/`clippy`/`fmt`/`architecture`.

Die Fundstelle bleibt bewusst unbenannt: sie steht im Compiler-Output, nicht in diesem Ticket. Der
Befehl oben zählt sie jederzeit selbst auf, eine hier abgeschriebene Liste wäre am Tag der Umsetzung
schon veraltet.

## Acceptance criteria

- [ ] `cargo doc --no-deps --workspace` läuft **ohne eine einzige Warnung** durch.
- [ ] Keine Sichtbarkeit wurde erweitert, um einen Link aufzulösen: Port-Bridges, die privaten Module
      und die privaten Methoden/Konstanten bleiben privat. `cargo test -p architecture` bleibt grün.
- [ ] Die Erklärkraft der Doku bleibt erhalten — ein Kollaborateur, der vorher benannt war, ist
      weiterhin benannt (als Code-Auszeichnung oder umformulierte Prosa), nicht ersatzlos gestrichen.
- [ ] Eine Regressionssperre ist gesetzt: ein neuer Link von public Doku auf ein privates Item
      **schlägt fehl**, statt nur zu warnen — nachweisbar, indem man einen solchen Link probeweise
      einfügt und den Fehlschlag sieht.
- [ ] `cargo doc` ist in CLAUDE.md als Gate-Kommando dokumentiert, damit es zur Routine gehört.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

None - can start immediately.

Berührt keinen laufenden Slice: die Änderungen sind auf Doc-Kommentare und die Workspace-Lints
beschränkt, kein Verhalten ändert sich. Läuft darum konfliktarm neben den offenen Bau-Tickets.
