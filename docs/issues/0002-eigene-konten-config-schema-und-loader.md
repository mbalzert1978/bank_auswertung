---
id: "0002"
title: "Eigene-Konten-Config: Schema und Loader"
status: blocked
---

# Eigene-Konten-Config: Schema und Loader

**Typ:** HITL — braucht Markus' echte eigene IBANs + Konto-Rollen.

> **Umbau durch [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md).** Die
> ursprünglich hierfür gebaute Slice-Triade `eigene_konten_{domain,adapter,specs}` wurde
> **entfernt**: in der event-getriebenen Pipeline wird `Gegenkonto::Eigen|Fremd` **nicht**
> mehr inline am Eingang aufgelöst, sondern im nachgelagerten **`auswerten`-Slice** — und
> dort **auf Token** (Gegenkonto-`KONTO_x` gegen die mit demselben Seed tokenisierte Menge
> der eigenen Konten; `auswerten` sieht keine rohe IBAN mehr). Der **Config-Artefakt** unten
> (IBAN + Rolle je eigenem Konto, `eigene-konten.example.toml`) bleibt gültig und
> **wird zum `auswerten`-Input**; die Nachschlage-/Loader-Logik wird bei Bau von `auswerten`
> **token-orientiert neu** aufgesetzt (mod-97-Prüfung + `Rolle` als wiederverwendbarer Kern).
> Der Textkörper unten ist die ursprüngliche Slice-Fassung und bleibt als Kontext stehen.

## Was zu bauen ist

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

## Abnahmekriterien

- [x] Config-Format ist definiert und dokumentiert (Feld je eigenem Konto:
      IBAN + Rolle aus `{Giro, Haushalt, Spar, Vorlauf}`).
- [x] Loader liest eine Config-Datei ein und liefert eine typisierte
      In-Memory-Struktur; eine Datei mit unbekannter Rolle oder fehlerhafter
      IBAN wird abgelehnt (kein stillschweigendes Überspringen).
- [x] Nachschlagefunktion „gegeben eine Gegen-IBAN, ist sie eigen (und mit
      welcher Rolle) oder fremd?" ist getestet, inkl. Negativfall (fremde
      IBAN).
- [ ] Markus' echte eigene IBANs + Rollen sind erfasst und die Config damit
      befüllbar (mindestens eine reale Testkonfiguration liegt vor). — **offen
      (HITL)**; die Vorlage `eigene-konten.example.toml` (fiktive Platzhalter)
      liegt im Wurzelverzeichnis, die reale `eigene-konten.toml` ist gitignored
      und lokal von Markus zu befüllen.
- [x] Test belegt: dieselbe Config-Instanz beantwortet sowohl die
      Eigen/Fremd-Frage als auch die Rollen-Frage — eine Quelle, keine zwei
      separaten Listen.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
  (Domänen-Skelett als Voraussetzung des Slice-Aufbaus; die IBAN bleibt hier
  bewusst reine Adapter-Konvention, **nicht** der Domänen-`KontoId`).
