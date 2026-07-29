---
id: "0083"
title: "Lauf-Anstoß: Binary reiht den Auslöser ein und dreht die Broker-Loop"
status: blocked
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

## Acceptance criteria

- [ ] Ein Lauf gegen eine Beispiel-CSV füllt alle vier Ergebnis-Tabellen und endet mit leerer Outbox und Exit-Code 0.
- [ ] Ein Integrationstest fährt diesen Lauf end-to-end ohne einen einzigen Fake.
- [ ] Ein Test nagelt fest, dass das Lauf-Protokoll weder Rohwert, Freitext, IBAN noch Seed enthält.
- [ ] Ein terminal gescheiterter Lauf endet mit Exit-Code ungleich 0 und nennt den redigierten Fehler.
- [ ] Der `architecture`-Test hält fest, dass die Bin-Crate `io` als einzige interne Abhängigkeit bindet.
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- [0002](0002-eigene-konten-config-schema-und-loader.md) — die `auswerten`-Verdrahtung braucht
  die Eigene-Konten-Config als Wert; ohne ihren Loader kann das Programm sie nicht beschaffen.
