# Compiler-getriebenes Implementieren mit AI-Tools

Wie AI-Tools (Claude Code, Codex) Rusts Compiler als treibende Feedback-Schleife nutzen, um
sich zur korrekten Implementierung zu iterieren. Jede Regel ist aus dem Artikel
[„Coding Rust with Claude Code and Codex" (tigranbs, dev.to)](https://dev.to/tigranbs/coding-rust-with-claude-code-and-codex-2c4j)
belegt.

## Compiler-Fehler als treibende Autorität

Rusts Compiler als automatischen Reviewer nutzen — nach jeder AI-Änderung `cargo check`
laufen lassen und den konkreten Fehler die nächste Änderung treiben lassen.

Do:

```text
Änderung → cargo check → konkreter Fehler (z. B. E0106) → AI restrukturiert → erneut cargo check
```

Don't:

```text
viele Änderungen sammeln, erst am Ende kompilieren  # verliert die enge Feedback-Schleife, Fehlerursache unklar
```

## Strukturierte Fehlerausgabe unverändert zurückspielen

Die vollständige, strukturierte Compiler-Fehlermeldung (Fehlercode, exakte Position,
Erklärung, vorgeschlagener Fix) unverändert ans AI-Tool zurückgeben, das sie parsen und
darauf handeln kann.

Do:

```text
error[E0106]: missing lifetime specifier
 --> src/ws.rs:12:20
 help: consider introducing a named lifetime   ← komplette Meldung an die AI geben
```

Don't:

```text
"kompiliert nicht, bitte fixen"  # wirft das deterministische Signal (Code/Position/Fix) weg
```

## Traits und Typen zuerst

Zuerst Traits und Typen — die klaren Grenzen — definieren, dann die AI die Logik
implementieren lassen.

Do:

```rust
pub trait MessageHandler {
    fn handle(&self, msg: WsMessage) -> Result<Reply, HandlerError>;
}
// erst der Vertrag, dann füllt die AI die Implementierung
```

Don't:

```rust
// AI schreibt die Logik ohne fixierte Signaturen — Grenzen unklar, der Compiler kann nichts erzwingen
fn handle_everything(input: &str) -> String { /* rät sich die Form zusammen */ }
```

## cargo clippy aggressiv einsetzen

`cargo clippy` aggressiv mit allen Lints nutzen — mehr Feedback bedeutet besseren AI-Output.

Do:

```bash
cargo clippy --all-targets -- -W clippy::all -W clippy::pedantic
```

Don't:

```bash
cargo build   # clippy weggelassen / nur Default-Lints: weniger Feedback ⇒ schwächerer AI-Output
```

## Strenge Checks in der CI

In der CI strenge Checks erzwingen, damit fehlerhafte AI-Edits nicht durchrutschen.

Do:

```bash
cargo check && cargo clippy -- -D warnings && cargo test   # Gate: Warnung = Fehlschlag
```

Don't:

```bash
cargo build || true   # „grün, wenn es baut" ignoriert vom Compiler markierte Probleme
```

## CLAUDE.md als Leitplanke pflegen

Eine CLAUDE.md pflegen, die Projektstruktur, Konventionen und Best Practices dokumentiert,
damit AI-Tools mit hoher Sicherheit arbeiten.

Do:

```text
CLAUDE.md: Projektstruktur + Konventionen + Best Practices  → AI operiert mit hoher Confidence
```

Don't:

```text
kein Projektdokument  # die AI rät Konventionen und trifft inkonsistente Entscheidungen
```

## Mit klar umrissenen Aufgaben starten

Mit gut definierten, klar umrissenen Aufgaben starten.

Do:

```text
Aufgabe: "Implementiere MessageHandler für WsMessage::Ping → Pong-Reply"   ← scharfe Grenzen
```

Don't:

```text
Aufgabe: "mach das WebSocket-Zeug besser"  # unscharfer Auftrag, die AI läuft in die falsche Richtung
```

## Vertrauen, aber verifizieren

Den AI-Edits vertrauen, sie aber verifizieren („verify but trust").

Do:

```text
AI-Edit übernehmen → durch Compiler / Clippy / Tests verifizieren, bevor gemergt wird
```

Don't:

```text
AI-Output ungeprüft nach main mergen  # ohne Verifikation kein Sicherheitsnetz
```
