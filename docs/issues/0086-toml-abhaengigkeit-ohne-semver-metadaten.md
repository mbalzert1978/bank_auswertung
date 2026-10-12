---
id: "0086"
title: "toml-Abhängigkeit ohne Semver-Metadaten"
status: open
---

# toml-Abhängigkeit ohne Semver-Metadaten

**Typ:** AFK

> GitHub-Issue: [#103](https://github.com/mbalzert1978/bank_auswertung/issues/103)

## What to build

Die `toml`-Abhängigkeit von `io` ist mit Build-Metadaten hinter einem `+` angegeben. Cargo
ignoriert diesen Teil der Anforderung und warnt bei **jedem** Workspace-Kommando darüber —
auch bei `build`, `test` und `clippy`. Die Warnung ist harmlos, aber sie steht in jedem
Lauf über der eigentlichen Ausgabe und tarnt im Rauschen jede echte Warnung; das Repo hält
sonst konsequent `-Dwarnings`.

Die Versionsanforderung wird auf die reine Version gezogen, ohne die tatsächlich aufgelöste
Version zu wechseln.

## Acceptance criteria

- [ ] Kein Workspace-Kommando gibt die Semver-Metadaten-Warnung mehr aus.
- [ ] Die aufgelöste `toml`-Version bleibt unverändert (das Lockfile zeigt keinen Versionswechsel).
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- None — can start immediately.
