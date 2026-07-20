---
id: "0036"
title: fenstern-Triade — ein Auswertungsfenster materialisieren
status: closed
---

# fenstern-Triade — ein Auswertungsfenster materialisieren

**Typ:** AFK

## What to build

Die neue Slice-Triade **`fenstern`** (`_domain`/`_adapter`/`_specs`) nach dem Muster des
`obfuskierung`-Slice, mit ihrem Kern-Pfad — dem Tracer Bullet durch alle Schichten
([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md), Entscheidung A):

Das Kommando **`AdvanceWindow{anchor}`** trägt immer einen Anker. Der Fenster-Schritt liest die
obfuskierten Buchungen der Seite `[anchor − window_days + 1 … anchor]` über einen von der Domäne
geforderten Port, baut daraus **ein `Auswertungsfenster`-Aggregat** (echte N→1-Reduktion), legt es
in **eigener Ablage** ab — die Ablage vergibt die ID — und publiziert **die neue ID** als Zeiger:
`FensterGeschlossen{id}`. Weil dieser erste Schnitt **noch nicht paginiert**, endet die Kette
sofort und der Schritt publiziert zusätzlich **`HistorieVollständig`** mit der Lauf-Spanne
(die Fortsetzung kommt in [0037](0037-fenstern-selbst-fortsetzung-mehr-seiten-paging.md)).

Die **Invarianten sitzen an der Konstruktion** des Aggregats, nicht in einer Prüfung danach
(ADR-0005): **eine Währung** und **nicht-leer**. Lesen, Ablegen und Publizieren sind **je fehlbar
und terminal**; der Slice gibt seinen **nackten, redigierten** Domänen-Fehler zurück — kein
Dead-Letter, kein Retry im Slice (ADR-0012). Die Seitengröße `window_days` wird dem Handler als
**Wert hereingereicht** (die Config liest `io`) — genau wie der Seed bei `obfuskierung`; die Domäne
liest keine Datei.

Verifiziert wird **black-box über eine Fluent-TestApi** gegen In-Memory-Fakes.

## Acceptance criteria

- [ ] Eine Seite mit N obfuskierten Buchungen ergibt **genau ein** abgelegtes `Auswertungsfenster`
      und **genau ein** `FensterGeschlossen`, das dessen ID trägt.
- [ ] Ein `Auswertungsfenster` lässt sich **nicht leer** und **nicht aus gemischten Währungen**
      konstruieren — die illegalen Zustände sind unkonstruierbar, nicht „geprüft".
- [ ] Ein Fehlschlag beim Lesen, Ablegen **oder** Publizieren beendet den Lauf **terminal** mit dem
      nackten Domänen-Fehler.
- [ ] Die Fehlerbegründung spiegelt **nie** einen Rohwert (nur statischer Feldname bzw. opake ID).
- [ ] `window_days` kommt als Wert herein; die Domäne liest keine Datei und kennt keine Config.
- [ ] Die Specs sind reine Black-Box über TestApi + DTOs; das `_domain`-Crate ist dependency-frei
      und kein Slice greift auf ein fremdes `_domain` zu.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0035](0035-architektur-doku-auf-realen-stand-ziehen.md) — die Doku muss den realen
Crate-/Pipeline-Stand nennen, bevor ein Agent die neue Triade danach baut.
