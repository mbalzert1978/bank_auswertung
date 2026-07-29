---
id: "0029"
title: "einlesen: terminal-fehlbares store/publish"
status: closed
---

# einlesen: terminal-fehlbares store/publish

## Kontext

Im einheitlichen Pipeline-Muster sind **alle drei** I/O-Schritte eines Slice — lesen,
ablegen, publizieren — fehlbar und terminal (kein Dead-Letter, kein Retry im Slice; der
Slice gibt seinen nackten terminalen Domänen-Fehler zurück, `io` surfacet ihn). `einlesen`
erfüllt das erst **halb**: die Beschaffung ist bereits fehlbar, aber **Ablage und
Veröffentlichung sind heute unfehlbar** (`void`) — mit der Begründung, Staging könne nicht
scheitern. Bei echtem Persistenz-Zugriff stimmt das nicht: die Ablage (die die Identität
vergibt) und das Einreihen des Ereignisses können terminal scheitern.

Der ID-Handoff selbst ist in `einlesen` bereits korrekt (die Ablage gibt die vergebene ID
zurück, genau sie wird publiziert) — dieses Issue macht nur die beiden Schritte **fehlbar**.

## What to build

Ablage- und Publish-Port des `einlesen`-Slice **fehlbar** machen (terminaler Slice-Fehler),
und den Handler die Fehler per `?` propagieren lassen — unter Erhalt der bestehenden
Batch-Atomarität.

- Der Ablage-Port gibt die vergebene ID **als `Result`** zurück; der Publish-Port ist
  fehlbar.
- Die Fehler-Summe des Handlers (`ReadError`) erhält eine **terminale** Variante für
  gescheiterte Ablage/Veröffentlichung.
- Der Handler propagiert diese Fehler per `?` (die bisherige seiteneffekt-only-Iteration
  wird zu einer fehler-propagierenden Form); die **Batch-Atomarität bleibt**: erst alle
  Aggregate bauen, dann ablegen + veröffentlichen — „einige durch, einige verworfen" bleibt
  unkonstruierbar.
- Die In-Memory-Fakes, die TestApi und die Blackbox-Specs ziehen mit.
- Docstrings **entrümpeln**: die „Ablage ist unfehlbar (kein `Result`)"-Begründung und jede
  „Dead-Letter"/„io klassifiziert Warning/Retry/Critical"-Formulierung raus, ausufernde
  Rationale raus (lebt im ADR), nur Vertrag/Verhalten. Keine `examples/`-Verweise.

Der Slice trägt seinen **eigenen** Fehlertyp — **kein** geteilter Fehler-Crate mit anderen
Slices.

## Acceptance criteria

- [ ] Ablage- und Publish-Port sind fehlbar; die Ablage gibt die vergebene ID als `Result`
      zurück.
- [ ] `ReadError` deckt den terminalen Ablage-/Publish-Fehler ab; der Handler propagiert ihn
      per `?`.
- [ ] Batch-Atomarität ist erhalten (alle Aggregate vor der ersten Ablage gebaut; erste
      kaputte Zeile bricht den ganzen Lauf ab, nichts abgelegt/publiziert).
- [ ] Keine „Ablage ist unfehlbar"- und keine „Dead-Letter"-Formulierung mehr im Slice;
      Docstrings entrümpelt; keine `examples/`-Verweise.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

None - can start immediately.
