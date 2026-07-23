---
id: "0059"
title: "Config-Loader (toml): Seed"
status: open
---

# Config-Loader (toml): Seed

**Typ:** AFK

> GitHub-Issue: [#56](https://github.com/mbalzert1978/bank_auswertung/issues/56)

## What to build

Ein kleiner, persistenzfreier toml-Adapter in `io`, der den **Obfuskierungs-Seed** lädt und ihn als
**Wert** in den `obfuskierung`-Slice reicht — exakt das Muster von
[0054](0054-config-loader-toml-fenster-config.md), nur für den Seed. Der Slice liest keine Datei; `io`
liest die Config und reicht den rohen `u64` herein (wie heute mit den Fakes, nur echt).

Damit steht der letzte hereingereichte Wert des `obfuskierung`-Laufs auf einer echten Quelle: gleicher
Seed → gleiche `PERSON_x`/`KONTO_x` über Importe hinweg
([ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md)).

**Fehlbar + redigiert:** eine fehlende/ungültige Config schlägt über einen `ConfigError` sauber fehl —
**kein Panic**, der Fehlertext zitiert **nie** einen Rohwert (weder Dateiinhalt noch Pfad noch den
gelesenen Seed), nur die statische Ursache.

**Abgrenzung:** dies ist **nur** der Lade-Wert-Loader. Die **persistente** Seed-Verwaltung (Seed je
obfuskiertem Aggregat mitschreiben für Rehydration) ist das separate, bestehende
[0008](0008-persistente-pseudonym-zuordnung.md) und bleibt hier unberührt.

## Acceptance criteria

- [ ] Ein toml-Loader in `io` liest den Seed und reicht ihn als `u64`-Wert in den `obfuskierung`-Consumer;
      der Slice bindet weiterhin keine Datei.
- [ ] Eine fehlende/ungültige Config → sauberer, **redigierter** `ConfigError` (kein Panic, kein Rohwert /
      Pfad / Seed im Fehler-Chain).
- [ ] Der Loader ist persistenzfrei (hängt **nicht** an der rusqlite-Stack-Entscheidung).
- [ ] Alle sechs Gates grün.

## Blocked by

- None — kann sofort starten (unabhängig von 0056–0058; reiner Wert-Loader wie 0054).
