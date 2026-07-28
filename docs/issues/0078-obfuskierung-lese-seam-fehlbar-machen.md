---
id: "0078"
title: obfuskierung-Lese-Seam fehlbar machen
status: closed
---

# obfuskierung-Lese-Seam fehlbar machen

Setzt [ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) auf
der **Leseseite** des `obfuskierung`-Slice durch. **AFK.**

## What to build

Der Lese-Seam des `obfuskierung`-Slice ist der einzige von vier ohne Fehlerkanal: er gibt
die gefundene Zeile als `Option` zurück, und die io-Erfüllung verschluckt dabei jeden
Datenbank-Fehler zu einem „nichts gefunden". Eine fehlende Roh-Tabelle oder eine kaputte
Datenbank meldet der Slice deshalb heute als *referenzierte Buchung nicht gefunden* —
sachlich falsch und für die Fehlersuche irreführend. ADR-0012 verlangt ausdrücklich, dass
**alle drei** I/O-Schritte jedes Slice fehlbar sind; das ist wortgleich das Problem, das
[0075](0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md) auf der
Schreibseite behoben hat.

Der Slice hebt den Lese-Seam auf die dreiwertige Form, die der `auswerten`-Lese-Seam
bereits trägt — `Err` = Beschaffung gescheitert, `Ok(None)` = liegt nichts vor,
`Ok(Some)` = gefunden —, ergänzt im Domänen-Fehler eine `SourceUnavailable`-Variante mit
durchgereichter, redigierter Diagnose (formgleich mit `fenstern` und `auswerten`), zieht
die Port-Bridge und die io-Erfüllung nach und weist beide Fälle black-box nach. Die
Redaktions-Pflicht der Diagnose schuldet weiterhin der erfüllende Seam: nur
Zugriffs-Mechanik, nie ein Zell- oder Feldwert.

## Acceptance criteria

- [ ] Ein **unerreichbarer** Lese-Speicher lässt den Slice terminal mit der durchgereichten,
      redigierten io-Diagnose scheitern — nicht mehr als „nicht gefunden".
- [ ] Eine schlicht **absente** ID scheitert unverändert terminal als „nicht gefunden"; die
      beiden Fälle sind am Domänen-Fehler unterscheidbar.
- [ ] Der Lese-Seam trägt dieselbe dreiwertige Ergebnisform wie der `auswerten`-Lese-Seam;
      die io-Erfüllung verschluckt keinen Datenbank-Fehler mehr.
- [ ] Die Black-Box-Spec deckt **beide** Fälle über die TestApi ab, und die Diagnose enthält
      keinen Rohwert (keine IBAN, kein Name, kein Zellwert).
- [ ] Alle sechs Gates grün (`/cargo-six-gates`).

## Blocked by

None - can start immediately.
