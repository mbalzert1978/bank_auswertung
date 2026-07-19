# Bank-Auswertung

Lokale, wartbare Rust-Software, die Kontoumsätze einliest und die Hauptkostentreiber
und realistische Sparpotenziale aufdeckt. **Reine Analyse** — löst keine Transaktionen
aus, ändert keine Kontodaten.

## Umfang

- **Drin:** CSV-CAMT v8, Mehrkonten, Umbuchung-Netting, Gebühren, Wiederkehr (inkl.
  Jahreskosten und Preissteigerungen), Kategorisierung via Chat, gelernte Korrekturen,
  dynamischer Zeitraum (was die CSV hergibt), HTML-Report.
- **Bewusst draußen (YAGNI — später jeweils nur ein Adapter):** XML-camt, MT940,
  API-Kategorisierer, andere Report-Formate.

## Datenschutz

Lokale Verarbeitung. Den Rechner verlassen **nur** obfuskierte, deduplizierte
Fingerabdrücke samt gescrubbtem Freitext. Strukturelle Identifikatoren (eigene
IBAN/Kontonummer, Gegen-IBAN, BIC) **nie** — compilergeprüft. Privatnamen werden
pseudonymisiert; in Freitext eingebettete Identifikatoren werden best-effort gescrubbt
— **heuristisch, nicht beweisbar vollständig**, und genau das behaupten wir bewusst
nicht. Die Gläubiger-ID (öffentlicher Firmen-Schlüssel) bleibt als Analyse-Anker.

## Wo was steht

Diese README beschreibt **nur Zweck, Umfang und das Datenschutz-Versprechen** — bewusst
**ohne** Architektur-, Crate- oder Ablaufdetails, damit sie nicht veraltet. Maßgeblich
sind stattdessen:

| Frage | Quelle |
| --- | --- |
| Welche Begriffe gelten? (Ubiquitous Language) | [CONTEXT.md](CONTEXT.md) |
| Wie ist die Architektur geschnitten — und warum? | [docs/adr/](docs/adr/) |
| Was wird in welcher Reihenfolge gebaut, wie ist der Stand? | [CLAUDE.md](CLAUDE.md) |
| Produkt-Anforderungen und User Stories | [docs/prd/0001-bank-auswertung.md](docs/prd/0001-bank-auswertung.md) |
| Was ist offen, was kommt als Nächstes? | [docs/issues/PROGRESS.md](docs/issues/PROGRESS.md) |
