# Sprachkonvention

Projektsprache ist **Deutsch**. Die Rust-idiomatischen Bezeichner-Regeln (Domänen-Begriffe
folgen der ubiquitären Sprache) bleiben davon unberührt.

## Docs und Tests auf Deutsch, Infrastruktur-Code auf Englisch

Doc-Kommentare und Testnamen auf Deutsch; Mechanik-/Infrastruktur-Bezeichner auf Englisch
(Domänen-Typen tragen ihre deutschen Fachbegriffe, z. B. `Betrag`, `Auswertungsfenster`).

Do:

```rust
/// Reduziert die Zeilen eines Fensters auf ein Aggregat.
#[test]
fn fehlendes_pflichtfeld_laesst_den_batch_terminal_scheitern() { /* … */ }
```

Don't:

```rust
/// Reduces the lines of a window into an aggregate.
#[test]
fn missing_field_fails_batch() { /* englischer Doc-Kommentar + Testname */ }
```

## Antworten an den Nutzer auf Deutsch

Konversation/Narration gegenüber dem Nutzer auf Deutsch; nur Code/Bezeichner bleiben
Englisch bzw. deutsch-fachsprachlich wie oben.

Do:

> „Die sechs Gates sind grün — `einlesen` läuft jetzt ohne Fake."

Don't:

> "All six gates are green — `einlesen` now runs without a fake."
