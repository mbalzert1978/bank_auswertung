---
id: "0054"
title: "Config-Loader (toml): Fenster-Config"
status: open
---

# Config-Loader (toml): Fenster-Config

**Typ:** AFK

## What to build

Ein kleiner, persistenzfreier toml-Adapter in `io`, der die **Fenster-Config** lädt und sie als
**Werte** in den `fenstern`-Slice reicht — der Slice liest keine Datei und bindet keinen Kalender
([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)):

- `WindowDays` (die Seitengröße des Pagings),
- die **Tag-Stamp-Achse** (der Buchungstag-Wert, auf dem `fenstern` paginiert).

`io` liest die Config, der Slice bekommt fertige Werte — die Verortung „io liest die Config, die Domäne
bindet keinen Kalender" bleibt exakt wie heute mit den Fakes, nur echt.

**Abgrenzung:** die **token-orientierte Eigene-Konten-Config** (IBAN + Rolle Giro/Haushalt/Spar/Vorlauf,
zu Tokens geladen) ist ein **separates, bestehendes** Ticket
([0002](0002-eigene-konten-config-schema-und-loader.md)) — dieses Ticket deckt **nur** die
Fenster-Config und ersetzt 0002 nicht. Persistenzfrei — hängt **nicht** an der Stack-Entscheidung.

## Acceptance criteria

- [ ] Ein toml-Loader in `io` liest `WindowDays` + Tag-Stamp-Achse und reicht sie als Werte in
      `fenstern`; der Slice liest weiterhin **keine** Datei.
- [ ] Eine fehlende oder ungültige Config schlägt sauber und **redigiert** fehl (kein Panic, kein
      Rohwert im Fehler).
- [ ] Die toml-Dep liegt **nur in `io`**; `cargo test -p architecture` bleibt grün.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
      `cargo test -p architecture` und `cargo doc --no-deps --workspace` sind grün.

## Blocked by

None - can start immediately. (Persistenzfrei; hängt **nicht** an
[0049](0049-persistenz-stack-rusqlite-synchron-kein-sqlx.md).)
