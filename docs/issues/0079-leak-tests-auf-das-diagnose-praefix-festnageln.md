---
id: "0079"
title: Leak-Tests auf das Diagnose-Praefix festnageln
status: open
---

# Leak-Tests auf das Diagnose-Praefix festnageln

Härtet das Regressionsnetz hinter der Privacy-Invariante aus
[ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) (Punkt 5). **AFK.**

## What to build

Die Leak-Tests der io-Stores von `einlesen` und `obfuskierung` sichern heute **nur negativ**
zu: die Fehlermeldung enthält den bekannten Roh-Wert der Fixture nicht. Damit hängt der Test
allein an der Abwesenheit einer bestimmten Zeichenfolge — fiele die durchgereichte Diagnose
ganz weg oder käme sie aus einer anderen Quelle, bliebe er grün. Genau das ist aber der
Regress, den [0075](0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md)
verhindert hat, und dessen drittes Akzeptanzkriterium ruht auf diesem Test.

Der Slice ergänzt die Zusicherung um den **positiven** Teil: die Meldung trägt das erwartete
Store-Präfix, *und* sie enthält keinen Rohwert. Damit fällt der Test sowohl, wenn ein
Zellwert durchschlägt, als auch, wenn die Diagnose verschwindet. Beide Stores werden gleich
gezogen, weil `obfuskierung` die Form von `einlesen` übernommen hat und eine einseitige
Härtung die nächste Drift wäre.

## Acceptance criteria

- [ ] Der `obfuskierung`-Leak-Test sichert **positiv** das erwartete Store-Präfix der
      Diagnose **und** negativ die Abwesenheit des Rohwerts zu.
- [ ] Der `einlesen`-Leak-Test, von dem die Form stammt, ist gleich nachgezogen.
- [ ] Entfernt man die durchgereichte Diagnose probeweise aus dem jeweiligen Store, werden
      **beide** Tests rot (Mutationsprobe im Ticket dokumentiert, nicht eingecheckt).
- [ ] Alle sechs Gates grün (`/cargo-six-gates`).

## Blocked by

None - can start immediately.
