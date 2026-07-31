---
id: "0083"
title: "Lauf-Anstoß: Binary reiht den Auslöser ein und dreht die Broker-Loop"
status: closed
---

# Lauf-Anstoß: Binary reiht den Auslöser ein und dreht die Broker-Loop

**Typ:** AFK

## What to build

Die vier Slices sind gebaut und real verdrahtet, aber es gibt **kein Programm**: die Pipeline
läuft ausschließlich in Tests. Dieser Slice macht aus der Bibliothek einen Lauf.

Eine eigene Bin-Crate unter `crates/` — sie bindet als einzige interne Abhängigkeit `io` —
startet einen vollständigen Pipeline-Lauf gegen echte Eingaben: sie öffnet die
SQLite-Datei, liest Fenster-, Seed- und Eigene-Konten-Config über die bestehenden Loader,
registriert alle Consumer der Kette am Dispatcher, reiht den `ReadTransactions`-Auslöser als
Nachricht in die Outbox und dreht die Broker-Loop, bis keine offene Nachricht mehr bleibt.
Der Auslöser wird **eingereiht, nicht aufgerufen** — der Slice bleibt ein reiner
Outbox-Consumer, die Kopplung läuft weiter nur über die Outbox (ADR-0008).

Während des Laufs schreibt das Programm ein **redigiertes Lauf-Protokoll** auf den
Fehlerkanal: je verarbeiteter Nachricht ihr Nachrichtentyp, am Ende die Zahl je Typ, und bei
einem terminalen Fehlschlag dessen `Display` — **nie** ein Rohwert, **nie** Freitext, **nie**
eine IBAN, **nie** der Seed. Die Privacy-Invarianten gelten für die Ausgabe genauso wie für
jedes Egress-DTO.

Der Prozess endet mit Exit-Code 0, wenn die Outbox leer läuft, und mit einem Code ungleich 0,
sobald eine Nachricht terminal scheitert. Der Dependency-Richtungs-Test bekommt die Regel für
die neue Bin-Crate dazu, damit ihre eine erlaubte Kante festgenagelt ist.

**Testdaten sind und bleiben erfunden.** Kein Test — auch dieser nicht — läuft je gegen einen
echten Kontoauszug: die Eingabe ist eine **frei erfundene** CSV mit erfundenen Konten,
Empfängern und Beträgen (`lauf.csv` im `einlesen`-CSV-Fixture-Bestand). Nur so ist
strukturell ausgeschlossen, dass echte Kontodaten je in Repository, Testlauf oder
Fehlerausgabe geraten. Zu unterscheiden davon ist der **Adapter**-Begriff: der Lauf benutzt
die **realen** `io`-Adapter (echte SQLite-Datei, echte CSV-Quelle, echte Loader), keinen
Fake-Seam — real ist die Mechanik, erfunden sind die Daten.

## Acceptance criteria

- [x] Ein Lauf gegen die erfundene Lauf-CSV füllt alle vier Ergebnis-Tabellen und endet mit leerer Outbox und Exit-Code 0.
- [x] Ein Integrationstest fährt diesen Lauf end-to-end über die realen `io`-Adapter, ohne einen einzigen Fake-Seam — auf erfundenen Daten.
- [x] Ein Test nagelt fest, dass das Lauf-Protokoll weder Rohwert, Freitext, IBAN noch Seed enthält.
- [x] Ein terminal gescheiterter Lauf endet mit Exit-Code ungleich 0 und nennt den redigierten Fehler.
- [x] Der `architecture`-Test hält fest, dass die Bin-Crate `io` als einzige interne Abhängigkeit bindet.
- [x] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- ~~[0002](0002-eigene-konten-config-schema-und-loader.md)~~ — **aufgelöst**: der io-seitige
  Eigene-Konten-Loader (`io::config::EigeneKontenConfig`) ist mit diesem Ticket gebaut; er liest die
  IBANs, prüft Form + mod-97 und gibt **Token** heraus. Offen bleibt an 0002 allein das HITL-Stück
  (Markus' echte IBANs) — das blockiert den Lauf nicht, er läuft auf der erfundenen Config.

## Wie es gebaut wurde

- **`crates/lauf`** (Bin `bank-auswertung`) — reine Kommandozeile: fünf Pflicht-Optionen (`--db`,
  `--csv`, `--fenster`, `--seed`, `--eigene-konten`), Exit-Code `0`/`1`/`2` (Erfolg / terminal
  gescheitert / falsch aufgerufen). Bindet `io` als einzige interne und **keine** externe Abhängigkeit.
- **`io::run`** — die Verdrahtung: Configs laden, sieben Consumer registrieren, Auslöser einreihen,
  Broker-Schleife drehen, redigiertes Protokoll schreiben. Der Test dazu liegt crate-intern
  (er prüft die vier Ergebnis-Tabellen und braucht dafür den privaten Verbindungszugriff).

### Zwei Befunde, die das Ticket nicht vorhersah

1. **Zwischen Import und Analyse fehlt eine Nachricht.** `obfuskieren` publiziert `Obfuscated`, aber
   **niemand** konsumiert es — `fenstern` hängt an `StartAnalysis`, das **niemand** publiziert.
   Gelöst mit zwei Phasen: der Lauf reiht `ReadTransactions` ein, lässt leer laufen, reiht dann
   `StartAnalysis` ein. `Obfuscated` räumt ein `AckSink` ab, sonst bliebe es ewig offen und jeder
   spätere Lauf stellte den Altbestand erneut zu.
2. **Register und Orakel hatten keine Erfüllung.** `io::auswerten::EmptyCategoryRegistry` /
   `UnansweredCategorizer` antworten bewusst leer statt zu scheitern — ADR-0004s Lackmustest
   („ohne LLM bleibt das Programm wertvoll") in Codeform. Sie treten ab, wenn 0009/0011 landen.
