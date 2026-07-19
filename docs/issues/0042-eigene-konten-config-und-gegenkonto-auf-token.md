---
id: "0042"
title: Eigene-Konten-Config und Gegenkonto Eigen/Fremd auf Token
status: blocked
---

# Eigene-Konten-Config und Gegenkonto Eigen/Fremd auf Token

**Typ:** AFK (die **realen** IBANs + Rollen bleiben HITL über [0002](0002-eigene-konten-config-schema-und-loader.md))

## What to build

Die Auflösung **`Gegenkonto::Eigen|Fremd`** wandert nach `auswerten` und arbeitet **auf Token**
([ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md), Entscheidung F):
das Gegenkonto-Token wird gegen die **mit demselben Seed tokenisierte Menge der eigenen Konten**
geprüft. `auswerten` liegt hinter der Datenschutz-Grenze und sieht **keine rohe IBAN** mehr.

Dafür wird die **Eigene-Konten-Config** aus [0002](0002-eigene-konten-config-schema-und-loader.md)
(je eigenem Konto: IBAN + **Konto-Rolle** Giro/Haushalt/Spar/Vorlauf) **token-orientiert neu**
aufgesetzt und dem Handler als **Wert hereingereicht** — die Domäne liest keine Datei. Die **Rolle
bleibt erhalten**, weil sie die Deutung trägt: ein Zufluss aufs Spar-Konto ist Sparen, nicht Konsum.

Das Ergebnis ist ein **Summentyp** (`Eigen(Rolle)` / `Fremd`), keine Kombination aus Flag und
Optionalfeld — illegale Zustände sind unkonstruierbar (ADR-0005).

Dieses Ticket liefert die **Auflösung**; das Netting darauf ist
[0004](0004-umbuchung-netting.md).

## Acceptance criteria

- [ ] Ein Gegenkonto, dessen Token in der tokenisierten Eigen-Menge liegt, wird als **Eigen samt
      Rolle** aufgelöst; jedes andere als **Fremd**.
- [ ] `auswerten` erhält zu keinem Zeitpunkt eine **rohe IBAN** — die Eigen-Menge liegt tokenisiert vor.
- [ ] Gleicher Seed → dieselbe Tokenisierung der eigenen Konten; ein anderer Seed bricht die
      Zuordnung (und das ist sichtbar, nicht still).
- [ ] Eine Config mit **unbekannter Rolle** oder **fehlerhafter IBAN** wird abgelehnt — kein
      stillschweigendes Überspringen.
- [ ] Die Config wird als Wert hereingereicht; die Domäne liest keine Datei.
- [ ] Das Ergebnis ist ein Summentyp; „Fremd mit Rolle" ist unkonstruierbar.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md) — die `auswerten`-Triade
muss stehen, in der die Auflösung lebt.
