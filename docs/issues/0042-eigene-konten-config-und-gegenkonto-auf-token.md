---
id: "0042"
title: Eigene-Konten-Config und Gegenkonto Eigen/Fremd auf Token
status: closed
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

- [x] Ein Gegenkonto, dessen Token in der tokenisierten Eigen-Menge liegt, wird als **Eigen samt
      Rolle** aufgelöst; jedes andere als **Fremd**.
- [x] `auswerten` erhält zu keinem Zeitpunkt eine **rohe IBAN** — die Eigen-Menge liegt tokenisiert vor.
- [x] Gleicher Seed → dieselbe Tokenisierung der eigenen Konten; ein anderer Seed bricht die
      Zuordnung (und das ist sichtbar, nicht still) — abgesichert über den **opaken Seed-Marker**,
      s. u.
- [x] Eine Config mit **unbekannter Rolle** wird abgelehnt — kein stillschweigendes Überspringen.
      Die **IBAN** prüft dieser Slice **nicht**, s. „Abgrenzung" unten.
- [x] Die Config wird als Wert hereingereicht; die Domäne liest keine Datei.
- [x] Das Ergebnis ist ein Summentyp; „Fremd mit Rolle" ist unkonstruierbar.
- [x] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Abgrenzung: die IBAN-Prüfung liegt nicht hier

Das ursprüngliche Kriterium „eine Config mit **fehlerhafter IBAN** wird abgelehnt" ist **kein
Kriterium dieses Slice** — es widerspräche dem Kriterium eine Zeile darüber. Die IBAN existiert
legitim nur **vor** der Tokenisierung: der Loader ([0002](0002-eigene-konten-config-schema-und-loader.md))
besitzt sie, prüft sie und tokenisiert sie mit **demselben `Obfuscator`/Seed**, mit dem
`obfuskieren` die Gegenkonto-Token bildet. Was `auswerten` erreicht, ist ein Token — der Beweis ist
geführt, der Typ trägt ihn (_parse, don't validate_). Ihn hier ein zweites Mal führen zu wollen wäre
nicht nur redundant, sondern unmöglich: es gibt keine IBAN mehr zu prüfen. Das Kriterium wandert
damit unverändert zu **0002**.

Ein **zweiter** Tokenisierungsweg wurde bewusst **nicht** gebaut: es gibt genau einen `Obfuscator`
(in `obfuskierung`, io-erfüllt), und die Eigen-Menge muss aus genau ihm stammen.

## Nachtrag: der Seed-Marker

Mit reinen Token wäre eine falsch-seedige Eigen-Menge **lautlos** — kein Token matchte, jede
Umbuchung liefe als „fremd" durch, und der Bericht wiese sie als Konsum aus, ohne dass irgendetwas
fehlschlüge. Das Kriterium verlangt aber ausdrücklich *sichtbar, nicht still*.

Fenster und Config tragen darum je einen **opaken Seed-Marker** — **nicht** den Seed (der erreicht
diesen Slice nie, ADR-0010), sondern ein nicht umkehrbares Etikett seiner Identität. Stimmen die
beiden nicht überein, endet der Lauf **terminal und redigiert** (`EvaluationError::SeedMismatch`,
ohne Marker-Werte im Text), statt ein falsches Ergebnis abzulegen. Das ist eine **Erweiterung des
Fenster-DTOs**, die io beim Bau der Zeile mitliefern muss.

## Blocked by

~~Blocked by [0040](0040-auswerten-triade-skelett-pro-fenster-reichweite.md)~~ — aufgelöst: die
`auswerten`-Triade steht (in `main` gemerged), die Auflösung lebt darin.
