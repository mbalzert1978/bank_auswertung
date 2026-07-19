---
id: "0041"
title: Fingerabdruck als reine Domänenfunktion
status: blocked
---

# Fingerabdruck als reine Domänenfunktion

**Typ:** AFK

## What to build

Der **Fingerabdruck** — der stabile Schlüssel, der eine _Art_ von Buchung identifiziert — als
**reine Funktion in der `auswerten`-Domäne**, nicht als Adapter-Aufgabe
([ADR-0004](../adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md),
[ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md) Entscheidung F).

Er entsteht aus dem **Empfänger**; bei einem **Zahlungsintermediär** (PayPal, Klarna, Amazon
Payments) zusätzlich aus einem **Händler-Token des Verwendungszwecks** — sonst fasst der Durchleiter
alle dahinterliegenden Händler zu einem Klumpen zusammen und die Wiederkehr hängt am falschen
Schlüssel. Bei **Privatpersonen** geht das seed-basierte **Pseudonym** ein, damit derselbe Mensch
über Importe hinweg denselben Fingerabdruck erzeugt (ADR-0010) — ohne preiszugeben, wer dahintersteckt.

Der Fingerabdruck ist die Achse für **drei** Dinge: die Wiederkehr-Erkennung, die gelernten
Korrekturen und die Deduplizierung der Kategorisierer-Anfrage. Deshalb ist er **Domänenwissen** —
entfernt man das LLM, bleibt er gültig.

## Acceptance criteria

- [ ] Zwei Buchungen desselben Händlers ergeben **denselben** Fingerabdruck.
- [ ] Zwei **verschiedene** Händler, die über **denselben** Zahlungsintermediär abgerechnet werden,
      ergeben **verschiedene** Fingerabdrücke (Test mit mindestens zwei Händlern hinter einem
      Intermediär).
- [ ] Eine Privatperson-Buchung erzeugt bei **gleichem Seed** über Importe hinweg denselben
      Fingerabdruck; ein anderer Seed ändert ihn.
- [ ] Die Funktion ist **pur** und liegt in der Domäne — kein Adapter, kein `io`, kein Datei-/
      Netzzugriff.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md) — die `auswerten`-Domäne,
in der der Fingerabdruck lebt, muss existieren.
