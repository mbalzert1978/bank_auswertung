---
id: "0002"
title: "Eigene-Konten-Config: Schema und Loader"
status: blocked
---

# Eigene-Konten-Config: Schema und Loader

**Typ:** HITL — braucht Markus' echte eigene IBANs + Konto-Rollen.

## What to build

Die **Eigene-Konten-Config** (lokal, adapter-seitig): je eigenem Konto seine
IBAN + Konto-Rolle (Giro/Haushalt/Spar/Vorlauf). Das ist die **eine Quelle**
für zwei spätere Verwendungen: die Auflösung `Gegenkonto::Eigen|Fremd` beim
CSV-Einlesen (Slice „CSV-CAMT-v8-Adapter") und die Rollen-Deutung der Analyse
(Cashflow je Konto). Beide dürfen nicht auseinanderlaufen — deshalb eine
gemeinsame Config statt zweier getrennter Listen.

Dieser Slice liefert: das Config-Format (z. B. TOML/YAML — IBAN + Rolle je
Eintrag), einen Loader, der die Datei einliest und in eine typisierte
Struktur überführt (IBAN bleibt hier noch reine Adapter-Konvention, nicht der
Domänen-`KontoId`), sowie eine Nachschlagefunktion „ist diese Gegen-IBAN
eigen, und wenn ja mit welcher Rolle?".

**Braucht echte Eingabedaten von Markus**: seine tatsächlichen eigenen IBANs
und deren Rollen (Giro/Haushalt/Spar/Vorlauf) — ohne diese Angabe ist der
Slice nicht abschließbar.

## Acceptance criteria

- [ ] Config-Format ist definiert und dokumentiert (Feld je eigenem Konto:
      IBAN + Rolle aus `{Giro, Haushalt, Spar, Vorlauf}`).
- [ ] Loader liest eine Config-Datei ein und liefert eine typisierte
      In-Memory-Struktur; eine Datei mit unbekannter Rolle oder fehlerhafter
      IBAN wird abgelehnt (kein stillschweigendes Überspringen).
- [ ] Nachschlagefunktion „gegeben eine Gegen-IBAN, ist sie eigen (und mit
      welcher Rolle) oder fremd?" ist getestet, inkl. Negativfall (fremde
      IBAN).
- [ ] Markus' echte eigene IBANs + Rollen sind erfasst und die Config damit
      befüllbar (mindestens eine reale Testkonfiguration liegt vor).
- [ ] Test belegt: dieselbe Config-Instanz beantwortet sowohl die
      Eigen/Fremd-Frage als auch die Rollen-Frage — eine Quelle, keine zwei
      separaten Listen.

## Blocked by

- Blocked by [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
  (nutzt `KontoId` als Domänen-Value-Object für die Rollen-Auflösung).
