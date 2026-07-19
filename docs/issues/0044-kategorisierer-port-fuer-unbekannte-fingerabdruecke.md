---
id: "0044"
title: Kategorisierer-Port für unbekannte Fingerabdrücke
status: blocked
---

# Kategorisierer-Port für unbekannte Fingerabdrücke

**Typ:** AFK

## What to build

Der **`Kategorisierer`** als von der Domäne **geforderter Port** — angefragt **nur** für
Fingerabdrücke, die die Domäne **noch nicht kennt**
([ADR-0004](../adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md),
[ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md) Entscheidung F). Das LLM
ist **Orakel, kein Pipeline-Schritt**: die Domäne entscheidet, wann sie fragt — Lackmustest: entfernt
man das LLM, bleibt das Programm wertvoll, nur automatische Labels für unbekannte Händler fehlen.

Der Port spricht **je Richtung sein eigenes schmales DTO** (ADR-0002): ausgehend
**`KategorieAnfrage`** (Fingerabdruck + obfuskierter Text), eingehend **`KategorieAntwort`**
(Fingerabdruck → Kategorie). Die Antwort schlüsselt auf den **Fingerabdruck, nie auf den Text** —
damit ist Re-Identifikation **strukturell** unmöglich, nicht bloß per Review verhindert.

Das **Egress-DTO** trägt die compilerfeste **„kein `Serialize`"-Sperre**: einen strukturellen
Identifikator in etwas einzubetten, das die Maschine verlässt, muss ein **Compile-Fehler** sein.
Die **Deduplizierung über Fingerabdrücke** hält den Chat-Payload klein.

## Acceptance criteria

- [ ] Nur der Domäne **unbekannte** Fingerabdrücke werden angefragt; bekannte lösen keine Anfrage aus.
- [ ] Das Egress-DTO ist **nicht serialisierbar** — die Sperre greift zur **Compile-Zeit**.
- [ ] Das Egress-DTO trägt **keinen** strukturellen Identifikator (keine IBAN/Kontonummer/BIC).
- [ ] Die Antwort wird über den **Fingerabdruck** zugeordnet, nicht über den Text.
- [ ] Mehrere Buchungen mit demselben Fingerabdruck erzeugen **eine** Anfrage, nicht mehrere.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0041](0041-fingerabdruck-als-domaenenfunktion.md) — „unbekannter Fingerabdruck" setzt
den Fingerabdruck voraus.
