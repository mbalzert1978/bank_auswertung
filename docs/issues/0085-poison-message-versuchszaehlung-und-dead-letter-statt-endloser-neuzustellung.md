---
id: "0085"
title: "Poison-Message: Versuchszählung und Dead-Letter statt endloser Neuzustellung"
status: open
---

# Poison-Message: Versuchszählung und Dead-Letter statt endloser Neuzustellung

**Typ:** AFK

> GitHub-Issue: [#102](https://github.com/mbalzert1978/bank_auswertung/issues/102)

## What to build

Eine Nachricht, deren Handler **terminal** scheitert, bleibt heute in der SQLite-Outbox
**offen** — jeder weitere Lauf stellt sie erneut zu und scheitert erneut. Dasselbe gilt für
einen wiederholt scheiternden Commit. Die Outbox-Mechanik in `io` benennt diese Lücke im
eigenen Doc-Kommentar selbst als „noch nicht gebaut": `OutboxError` kennt zwar bereits
`Retry`/`Warning`/`Critical`, aber niemand zählt Versuche und niemand nimmt eine Nachricht
je aus der Zustellung. Eine vergiftete Nachricht hält die Kette damit dauerhaft auf.

Die Outbox zählt Zustellversuche mit. Ein **terminaler** Fehlschlag (`Warning`/`Critical`)
nimmt die Nachricht **sofort** aus der Zustellung; ein **transienter** (`Retry`) erst,
nachdem eine konfigurierte Höchstzahl an Versuchen erschöpft ist. Aus der Zustellung
genommene Nachrichten bleiben mit ihrem **redigierten** Grund abfragbar, sodass ein Lauf sie
am Ende benennen kann, statt sie stillschweigend zu verlieren.

Das beantwortet die in
[ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) ausdrücklich
vertagte Frage, **wie `io` einen terminalen Fehlschlag surfacet**. Am Slice ändert sich
nichts: er gibt weiterhin nur seinen nackten, redigierten Domänen-Fehler zurück und kennt
weder Retry noch Dead-Letter.

## Acceptance criteria

- [ ] Ein terminal scheiternder Handler nimmt seine Nachricht beim ersten Fehlschlag aus der Zustellung; ein Folgelauf stellt sie nicht erneut zu.
- [ ] Ein transient scheiternder Commit wird bis zur konfigurierten Höchstzahl erneut zugestellt und danach aus der Zustellung genommen.
- [ ] Die aus der Zustellung genommenen Nachrichten sind mit ihrem Grund abfragbar; ein Test nagelt fest, dass der Grund keinen Rohwert und keinen Freitext trägt.
- [ ] Eine gesunde Nachricht hinter einer vergifteten wird weiterhin zugestellt — die Kette bleibt nicht stehen.
- [ ] Der öffentliche Vertrag Richtung Slice bleibt unverändert (kein Slice lernt Retry oder Dead-Letter kennen).
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- None — can start immediately.
