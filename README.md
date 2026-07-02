# Bank-Auswertung

Lokale, wartbare Rust-Software, die Kontoumsätze einliest und die
Hauptkostentreiber + realistische Sparpotenziale aufdeckt. Reine Analyse — löst
keine Transaktionen aus, ändert keine Kontodaten.

Begriffe: [CONTEXT.md](CONTEXT.md). Entscheidungen: [docs/adr/](docs/adr/).

## Architektur

> **Im Umbau ([ADR-0007](docs/adr/0007-modularer-monolith-feature-slices.md)):**
> modularer Monolith aus **Feature-Slices**. Jeder Slice ist eine Triade
> `<slice>_domain`/`_adapter`/`_specs`; dazu ein **dünner** interface-only
> `shared_kernel`. Es gibt **kein** geteiltes `domain`-Crate mehr — jeder Slice hat
> eigene Value Objects und gibt nach außen nur DTOs heraus. Die folgende
> Beschreibung nennt die Fachfunktionen und Ports; ihre Verteilung auf Slices und
> die Crate-Topologie regelt ADR-0007.

Hexagonal, Cargo-Workspace ([ADR-0001](docs/adr/0001-hexagonale-architektur.md),
[0002](docs/adr/0002-dto-grenze-und-anti-corruption-layer.md),
[0003](docs/adr/0003-rust-als-sprache.md),
[0004](docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md),
[0005](docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md),
[0006](docs/adr/0006-pseudonym-stabilitaet.md),
[0007](docs/adr/0007-modularer-monolith-feature-slices.md)):

- **Domänen-Crate (rein, kein I/O):** Wiederkehr-Erkennung (inkl. Jahreskosten &
  Preissteigerung), Umbuchung-Netting, Gebühren-Klassifikation, Aderlass/Cashflow
  je Konto & Monat, Pareto-Ranking, Spar-Kandidaten, Kategorie-Register +
  gelernte Korrekturen. Öffentliche Grenze = DTOs (Anti-Corruption-Layer), Tests
  docken dort an. Modell: Value Objects + Summentypen statt Primitiven/Flags,
  deklarativ-funktional
  ([ADR-0005](docs/adr/0005-domaenenmodell-value-objects-und-summentypen.md)).
- **4 Ports + Adapter:**
  - `Umsatzquelle` (rein) → CSV-CAMT-v8-Adapter zuerst. Obfuskiert beim Mappen,
    sodass die Domäne nur obfuskierte Value Objects erhält: IBAN/BIC bleiben im
    Adapter, `Auftragskonto`→`KONTO_x`, Privatperson→`PERSON_x`, Gegen-IBAN →
    `Gegenkonto::Eigen(Rolle)|Fremd` via Eigene-Konten-Config; Empfänger-/
    Verwendungszweck-Freitext wird zu `ObfuskierterText` gescrubbt.
  - `Kategorisierer` (raus) → spricht eigene schmale DTOs, **nicht** die
    Buchung-DTO: Hinweg `KategorieAnfrage` (Fingerabdruck + `ObfuskierterText`
    für Empfänger/Verwendungszweck), Rückweg `KategorieAntwort` (Fingerabdruck →
    Kategorie, schlüsselt auf dem Fingerabdruck, nie auf dem Text → keine
    Re-Identifikation). Der typisierte Obfuskierungs-Ausgang. Chat-Payload-Adapter
    zuerst (JSON nur der unbekannten Fingerabdrücke → Chat → zurück), API-Adapter
    später, gleicher Port.
  - `Speicher` (raus) → SQLite, speichert DTOs.
  - `Bericht` (raus) → HTML zuerst.

## Umfang

- **Drin:** CSV-CAMT v8, Mehrkonten, Umbuchung-Netting, Gebühren, Wiederkehr,
  Kategorisierung via Chat, gelernte Korrekturen, dynamischer Zeitraum (was die
  CSV hergibt), HTML-Report.
- **Bewusst draußen (YAGNI, später nur ein Adapter):** XML-camt, MT940,
  API-Kategorisierer, andere Report-Formate.

## Datenschutz

Lokale Verarbeitung. Den Rechner verlassen nur obfuskierte, deduplizierte
Fingerabdrücke samt gescrubbtem Freitext. Strukturelle Identifikatoren (eigene
IBAN/Kontonr., Gegen-IBAN, BIC) nie — compilergeprüft. In Empfänger-/
Verwendungszweck-Freitext eingebettete Identifikatoren werden best-effort
gescrubbt (heuristisch, nicht beweisbar vollständig), Privatnamen
pseudonymisiert, Mapping bleibt lokal. Die Domäne erhält keine personen- oder
kontobeziehbaren Identifikatoren — nur obfuskierte Value Objects; die
Gläubiger-ID (öffentlicher Firmen-Schlüssel) bleibt bewusst als Analyse-Anker.

## Baureihenfolge (Tracer-Bullet)

1. Cargo-Workspace + Domänen-Skelett: `Buchung` aus Value Objects (`Betrag`,
   `Richtung`, `Empfänger::Firma|Person`, `Gegenkonto::Eigen(Rolle)|Fremd`) plus der
   schmale `KategorieAnfrage`-Ausgangstyp (Freitext als `ObfuskierterText`). Test
   am DTO-Vertrag inkl. „Projektion `Buchung`→`KategorieAnfrage` trägt keinen
   strukturellen Identifikator". _(ersetzt das Python-Hello-World)_
2. CSV-CAMT-v8-Adapter gegen echte Beispiel-Datei → `Buchung`. Obfuskiert beim
   Mappen: IBAN/BIC bleiben im Adapter, `Auftragskonto`→`KONTO_x`,
   Privatperson→`PERSON_x`, Gegen-IBAN → `Gegenkonto::Eigen(Rolle)|Fremd` via
   Eigene-Konten-Config, Freitext → `ObfuskierterText` (best-effort gescrubbt).
   Adapter sichert eine einheitliche Währung zu.
3. Analyse-Kern: Wiederkehr + Gebühren + Aderlass + Pareto. Schon ohne LLM
   wertvoll (Lackmustest).
4. `Speicher` (SQLite) für DTOs + Kategorie-Register.
5. `Kategorisierer`-Chat-Payload-Adapter (Export → Chat → Import).
6. HTML-`Bericht`.
7. Iteration: gemeinsam durchgehen, Korrekturen lernen, Sparpotenziale benennen.

## Offen

- Echte CSV-CAMT-v8-Beispieldatei (ein Monat reicht) für Schritt 2.
- **Eigene-Konten-Config** (lokal): je eigenem Konto seine IBAN + Konto-Rolle
  (Giro/Haushalt/Spar/Vorlauf). Eine Quelle für `Gegenkonto::Eigen(Rolle)|Fremd`
  (Schritt 2) **und** die Rollen-Deutung der Analyse (Schritt 3) — die CSV trägt
  sie nicht. Verbindlich vor Schritt 2.
- **Persistente Pseudonym-Zuordnung** (lokal, verlässt den Rechner nie): stabile
  `PERSON_x`-Token über Importe hinweg, sonst bricht der Fingerabdruck von
  Personen-Daueraufträgen ([ADR-0006](docs/adr/0006-pseudonym-stabilitaet.md)).
  Verbindlich vor Schritt 4.
- Scrubbing-Regeln für `ObfuskierterText` (IBAN/BIC-Muster, bekannte
  Personen-Token), fail-safe — verbindlich vor Schritt 5.
