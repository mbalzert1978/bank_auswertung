# Domänenmodell: Value Objects statt Primitiven, Summentypen statt Flags, deklarativ-funktional

> **Status (seit [ADR-0007](0007-modularer-monolith-feature-slices.md)):
> gültig, neu verortet.** Value Objects, Summentypen und der deklarativ-funktionale
> Stil bleiben verbindlich — aber je **Slice-Domäne**, nicht in **einem** geteilten
> Modell. Namenskonvention neu: Domänentypen tragen `Entity`/`Aggregat`, die
> UBQL-DTOs am Rand den schlichten Glossar-Namen (ADR-0007).

Die Domäne benutzt **keine nackten Primitiven**. Jedes fachliche Konzept ist ein
Value Object (`Betrag`, `KontoId`, `Pseudonym`, `Fingerabdruck`, `GläubigerId`,
`ObfuskierterText`), das beim Konstruieren validiert — _parse, don't validate_.
Ein roher `String` oder `f64` kommt nicht über die DTO-Grenze in die Domäne;
geprüft wird einmal am Rand, danach trägt der Typ den Beweis. Auch der zum LLM
ausgehende Freitext ist kein `String`, sondern `ObfuskierterText` — sein
Konstruktor scrubbt eingebettete Identifikatoren (siehe
[ADR-0003](0003-rust-als-sprache.md)), sodass „ungescrubbt" gar nicht
serialisierbar ist.

Geschlossene Fallmengen sind **Summentypen** (Rust-`enum` / Discriminated Union)
mit erschöpfendem `match`, nie ein Flag plus optionale Felder. Beispiele:
`Richtung`, `Gegenkonto::Eigen(Rolle)|Fremd`, `Empfänger::Firma|Person`,
`Buchungsart`.
Eine ungültige Kombination ist damit **nicht konstruierbar** — nicht „wird
geprüft". Eine neue Variante zwingt der Compiler über die Exhaustiveness an jede
Stelle, die sie behandeln muss.

Der Stil ist **deklarativ und funktional**: reine Transformationen über
unveränderliche Werte, Collection-Pipelines (`map`/`filter`/`fold`) statt
händischer Schleifen mit Mutable-Akkumulatoren, `match` statt `if`/`else`-Kaskaden.
Seiteneffekte (I/O, Mutation) leben am Rand — in den Adaptern, nicht im Kern.

Bewusst in Kauf genommen: mehr Typen und mehr Konstruktor-Zeremonie. Akzeptiert,
weil jedes Value Object einen Invariant trägt und jedes `enum` eine ganze Klasse
von Bugs ausschließt, statt sie auf verstreute Laufzeit-Checks zu verteilen.

## Konsequenzen

- Ein neues Feld kommt als Value Object, nicht als `String`/Zahl. Reviews lehnen
  Primitiven an der Domänengrenze ab.
- Ein neuer Zustand kommt als zusätzliche `enum`-Variante; der Compiler weist via
  Exhaustiveness jede Stelle aus, die noch fehlt.
- Tests am DTO-Vertrag prüfen auch die Konstruktor-Validierung: illegaler Wert →
  kein Objekt (kein „Objekt da, aber innen kaputt").
- `Betrag` ist eine **vorzeichenlose** Größe mit Währung; das Vorzeichen lebt
  ausschließlich in `Richtung::Eingang|Ausgang`, nie unabhängig daneben (−5 € mit
  `Eingang` ist nicht konstruierbar). Gemischte Währungen summiert die Domäne
  nicht — der eingehende Adapter sichert eine einheitliche Währung zu.
