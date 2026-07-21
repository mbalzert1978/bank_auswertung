---
id: "0055"
title: "Währungs-bewusster Betrags-Decode: Minor Unit per ISO 4217 von der Domäne hoch"
status: blocked
---

# Währungs-bewusster Betrags-Decode: Minor Unit per ISO 4217 von der Domäne hoch

**Typ:** HITL

> **Nur ein Problem-Anriss.** Der konkrete Schnitt wird **bei Ausführung** ausgeplant (darum
> HITL); hier steht bewusst *was* falsch ist und *welche* Entscheidung ansteht, nicht das *wie*.

## What to build

Das Geld-Modell nimmt heute eine **feste Minor-Unit-Skala von 2** (Cent) an: der `MoneySeam`
dekodiert das Betragsfeld zu `DecodedAmount { cent, sign }`, **ohne die Währung zu kennen**, und
`Betrag` / `Line::amount_cent` tragen „Cent". Betrag (io-Seam) und Währung (`Currency::from_code`,
Domäne) sind **getrennt** — nichts kennt die Paarung, um die richtige Skala zu wählen.

Das ist für EUR korrekt, für Währungen mit anderer **ISO-4217-Minor-Unit falsch**: JPY hat 0
Nachkommastellen, BHD 3. Ein als „Cent" (Skala 2) gedeuteter JPY- oder BHD-Betrag ist um Faktor
100 bzw. 10 verschoben.

Diese Issue macht den Geldwert **währungs-bewusst von der Domäne hoch**: die Domäne besitzt Währung
**und** ihre Minor Unit (ISO 4217), der Betrag wird auf die Minor Units *dieser* Währung skaliert
(`1.234,56 EUR` → 123 456 @ Skala 2; `1234 JPY` → 1234 @ Skala 0; `1,234 BHD` → 1234 @ Skala 3).
Die Minor Unit stammt aus einer **battle-tested ISO-4217-Lib** in `io`, **kombiniert** mit der in
[0050](0050-money-calendar-decode-seams-real.md) gewählten Decimal-Lib — keine handgepflegte
Tabelle.

Bei Ausführung zu entscheiden (Anriss, nicht Plan): ändert sich die **Seam-Signatur** (Währung mit
hinein), oder wandert die Skalierung ganz in Domäne/Adapter; wie `amount_cent` / `Betrag` / `Line`
von „Cent" auf „Minor Units" umbenannt/umgeschnitten werden; wo die Währungs-Deutung
(`Currency::from_code`) und die Minor-Unit-Quelle zusammenkommen, ohne die Privacy-/DTO-Grenzen zu
verletzen.

## Acceptance criteria

- [ ] Der Geldwert trägt **währungs-korrekte Minor Units per ISO 4217** statt fester Skala 2; eine
      Nicht-2-Stellen-Währung (JPY = 0, BHD = 3) skaliert **end-to-end** korrekt (über den
      einlesen-Bridge-Weg verifiziert).
- [ ] Die Minor Unit kommt aus einer **battle-tested ISO-4217-Lib in `io`**, kombiniert mit der
      Decimal-Lib — keine handgepflegte Nachkommastellen-Tabelle; **neue Fremd-Deps nur in `io`**,
      die Domäne bleibt dependency-frei.
- [ ] Die **Domäne bleibt Wahrheit** für die Währung; Modell und Benennung spiegeln „Minor Units",
      nicht „Cent". Die feste Skala-2-Annahme aus [0050](0050-money-calendar-decode-seams-real.md)
      ist abgelöst.
- [ ] Die sechs Gates (`build`, `test`, `clippy --all-targets`, `fmt --check`,
      `test -p architecture`, `doc --no-deps --workspace`) sind grün.

## Blocked by

Blocked by [0050](0050-money-calendar-decode-seams-real.md) — verfeinert dessen bewusst
währungs-agnostischen `MoneySeam` (Skala 2); der reine Dekodier-Seam muss erst stehen.
