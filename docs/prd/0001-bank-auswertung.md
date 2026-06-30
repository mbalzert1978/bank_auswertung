# PRD: Bank-Auswertung

> **Entwurf (lokal).** Es existiert kein Issue-Tracker (kein Remote/GitHub-Repo);
> dieser PRD liegt als lokaler Entwurf unter `docs/prd/`. Veröffentlichung später,
> falls gewünscht.
>
> Quelle der Wahrheit: [README.md](../../README.md). Begriffe:
> [CONTEXT.md](../../CONTEXT.md). Entscheidungen: [docs/adr/](../adr/). Dieser PRD
> bildet die README treu ab und erfindet nichts darüber hinaus.

## Problem Statement

Aus Markus' Sicht: Monatliche Kosten ziehen seine Konten leer, und er sieht nicht
klar, wohin. Über mehrere Konten hinweg verschwimmt, was echter Abfluss ist und was
nur eine Umbuchung zwischen den eigenen Konten ist. Er will zweierlei: (1) wissen,
**wo das Geld wegblutet** — die Hauptkostentreiber —, und (2) **realistische
Sparpotenziale** finden. Gleichzeitig
sind das sensible Bankdaten: Er will sie lokal verarbeiten und nichts
Identifizierendes an externe Dienste geben.

## Solution

Eine lokale, wartbare Rust-Software liest die Kontoumsätze ein und macht **reine
Analyse** — sie löst keine Transaktionen aus und ändert keine Kontodaten. Sie
erkennt deterministisch wiederkehrende Abflüsse (Abos/Daueraufträge inkl.
Jahreskosten und Preissteigerung), nettet Umbuchungen zwischen eigenen Konten
heraus, klassifiziert und summiert Bank-Gebühren, rechnet den Aderlass/Cashflow je
Konto und Monat, rankt die größten Treiber nach Pareto und schlägt Spar-Kandidaten
vor. Die Kategorisierung unbekannter Händler übernimmt ein LLM — aber nur als
**Orakel** hinter einem Port, gefüttert ausschließlich mit obfuskierten,
deduplizierten Fingerabdrücken samt gescrubbtem Freitext. Strukturelle
Identifikatoren (IBAN/BIC/Kontonr.) verlassen den Rechner nie. Das Ergebnis ist
ein HTML-Report, den Markus mit Claude gemeinsam durchgeht, Korrekturen lernt und
Sparpotenziale benennt.

## User Stories

Akteure: **Kontoinhaber** (Markus, Endnutzer) und **Entwickler** (baut/wartet die
Software mit Claude).

### Einlesen & Mehrkonten

1. Als Kontoinhaber möchte ich meine Kontoumsätze aus dem CSV-CAMT-v8-Export
   einlesen, damit ich meine Ausgaben auswerten kann, ohne sie abzutippen.
2. Als Kontoinhaber möchte ich mehrere Konten zusammen auswerten, damit ich ein
   Gesamtbild über mein Mehrkontenmodell bekomme.
3. Als Kontoinhaber möchte ich den Auswertungszeitraum dynamisch aus dem nehmen,
   was die CSV hergibt, damit ich nicht an feste Zeiträume gebunden bin.

### Umbuchungen

1. Als Kontoinhaber möchte ich, dass Umbuchungen zwischen meinen eigenen Konten
   erkannt und aus der „Wo bluten wir"-Analyse herausgenettet werden, damit
   eigenes Geldverschieben nicht als Abfluss zählt.
2. Als Kontoinhaber möchte ich, dass Umbuchungen trotzdem sichtbar bleiben, damit
   ich die Geldflüsse zwischen meinen Konten nachvollziehen kann.

### Wiederkehr

1. Als Kontoinhaber möchte ich wiederkehrende Abflüsse (Abos, Daueraufträge)
   deterministisch erkannt bekommen, damit ich sehe, was mich regelmäßig kostet.
2. Als Kontoinhaber möchte ich, dass die Wiederkehr-Erkennung auch Jahreskosten
   erfasst, damit jährlich abgebuchte Abos nicht durchrutschen.
3. Als Kontoinhaber möchte ich Preissteigerungen bei wiederkehrenden Abflüssen
   erkannt bekommen, damit ich teurer gewordene Abos aufspüre.
4. Als Kontoinhaber möchte ich, dass ein Zahlungsintermediär (PayPal/Klarna/Amazon
   Payments) nicht alle Händler zu einem Klumpen zusammenfasst, damit die
   Wiederkehr am echten Händler hängt und nicht am Durchleiter.

### Gebühren

1. Als Kontoinhaber möchte ich Bank-Gebühren (z. B. Buchungstext `ENTGELT`)
   gesondert klassifiziert und summiert bekommen, damit ich sehe, was die Bank
   selbst einzieht.
2. Als Kontoinhaber möchte ich Umbuchungskosten — die Gebühr für eine Umbuchung —
   als echten Abfluss (Gebühr) ausgewiesen bekommen, anders als die Umbuchung
   selbst (die herausgenettet wird), damit ich diese Kosten nicht übersehe.

### Aderlass, Ranking, Sparen

1. Als Kontoinhaber möchte ich den Aderlass/Cashflow je Konto und Monat sehen,
   damit ich den Trend über die Zeit erkenne.
2. Als Kontoinhaber möchte ich ein Pareto-Ranking der größten Abflüsse, damit ich
   mich auf die wenigen großen Treiber konzentriere.
3. Als Kontoinhaber möchte ich strukturell gefundene Spar-Kandidaten
   vorgeschlagen bekommen, damit ich realistische Ansatzpunkte zum Sparen habe.

### Kategorisierung

1. Als Kontoinhaber möchte ich, dass Buchungen kategorisiert werden (z. B.
   Lebensmittel, Mobilität), damit ich meine Ausgaben thematisch verstehe.
2. Als Kontoinhaber möchte ich, dass die Kategorie bei Zahlungsintermediären aus
   dem Verwendungszweck und nicht aus dem Empfänger-Namen kommt, damit die
   Schublade stimmt.
3. Als Kontoinhaber möchte ich, dass nur der Domäne **unbekannte** Fingerabdrücke
   ans LLM gehen, damit Datenabfluss und API-Kosten minimal bleiben.
4. Als Kontoinhaber möchte ich meine Kategorie-Korrekturen lernen lassen, damit
   dasselbe Muster beim nächsten Mal richtig einsortiert ist.
5. Als Kontoinhaber möchte ich die Kategorisierung zuerst per Chat-Payload
   erledigen (Export → Chat → Import), damit ich ohne API-Anbindung starten kann.

### Report

1. Als Kontoinhaber möchte ich am Ende einen HTML-Report, damit ich die
   Auswertung übersichtlich vor mir habe und mit Claude durchgehen kann.
2. Als Kontoinhaber möchte ich, dass der Report die **Analyse der Domäne** zeigt
   und nicht „was das LLM sagte", damit die Aussagen belastbar sind.

### Datenschutz

1. Als datenschutzbewusster Kontoinhaber möchte ich, dass die Verarbeitung lokal
   läuft, damit meine Bankdaten meinen Rechner nicht verlassen.
2. Als datenschutzbewusster Kontoinhaber möchte ich, dass strukturelle
   Identifikatoren (eigene IBAN/Kontonr., Gegen-IBAN, BIC) den Rechner garantiert
   nie verlassen, damit ich nicht über Kontonummern identifizierbar bin.
3. Als datenschutzbewusster Kontoinhaber möchte ich, dass diese Garantie
   compilergeprüft ist und nicht von Review-Disziplin abhängt, damit ein Leak ein
   Compile-Fehler ist statt einer übersehenen Stelle.
4. Als datenschutzbewusster Kontoinhaber möchte ich, dass in
   Empfänger-/Verwendungszweck-Freitext eingebettete Identifikatoren best-effort
   gescrubbt werden, damit nicht über die Hintertür eine IBAN im Verwendungszweck
   rausgeht.
5. Als datenschutzbewusster Kontoinhaber möchte ich ehrlich wissen, dass das
   Freitext-Scrubbing heuristisch und nicht beweisbar vollständig ist, damit ich
   die Grenze des Modells kenne.
6. Als datenschutzbewusster Kontoinhaber möchte ich, dass Privatpersonen-Namen
   pseudonymisiert werden (`PERSON_x`) und das Mapping lokal bleibt, damit niemand
   sieht, wem ich privat Geld überweise.
7. Als datenschutzbewusster Kontoinhaber möchte ich, dass dasselbe Pseudonym über
   Importe hinweg stabil bleibt, damit Personen-Daueraufträge als Wiederkehr
   erkannt werden, ohne preiszugeben, wer dahintersteckt.
8. Als datenschutzbewusster Kontoinhaber möchte ich, dass bei Unsicherheit „Firma
   vs. Privatperson" fail-safe als Privatperson behandelt und pseudonymisiert
   wird, damit der teurere Fehler (Klartext-Leak eines echten Namens) nicht
   passiert.
9. Als Kontoinhaber möchte ich, dass die öffentliche Gläubiger-ID als
   Analyse-Anker erhalten bleibt, damit die Abo-Erkennung über die einziehende
   Firma funktioniert (sie identifiziert die Firma, nicht mich).

### Wartbarkeit (Entwickler)

1. Als Entwickler möchte ich, dass die Domäne keine Dateiformate und keinen
   LLM-Anbieter kennt, damit ich Formate und Dienste austauschen kann, ohne die
   Domäne anzufassen.
2. Als Entwickler möchte ich ein weiteres Eingabeformat als bloßen Adapter hinter
   dem `Umsatzquelle`-Port hinzufügen können, damit XML-camt/MT940 später keine
   Domänenänderung kosten.
3. Als Entwickler möchte ich die Domäne ohne Datei und ohne Netz testen können,
   damit die Analyse-Logik schnell und isoliert prüfbar ist.
4. Als Entwickler möchte ich die Domäne nur über öffentliche DTOs ansprechen
   (Anti-Corruption-Layer), damit das Innere frei refaktorierbar bleibt, solange
   der DTO-Vertrag hält.
5. Als Entwickler möchte ich, dass jeder Port seinen eigenen schmalen DTO spricht
   statt eines Gott-DTOs, damit der `Kategorisierer` nur Fingerabdruck +
   `ObfuskierterText` sieht.
6. Als Entwickler möchte ich, dass die Kategorie-Antwort auf dem Fingerabdruck
   schlüsselt und nie auf dem Text, damit Re-Identifikation strukturell
   ausgeschlossen ist.
7. Als Entwickler möchte ich illegale Zustände unrepräsentierbar machen (Value
   Objects + Summentypen statt Primitiven/Flags), damit ganze Bug-Klassen gar
   nicht erst konstruierbar sind.
8. Als Entwickler möchte ich, dass das Programm auch ohne LLM wertvoll ist
   (Lackmustest), damit die Domäne echt ist und nicht bloß ein LLM-Wrapper.
9. Als Entwickler möchte ich entlang vertikaler Slices (Tracer-Bullet-
   Baureihenfolge) bauen, damit nach jedem Schritt etwas Lauffähiges/Wertvolles
   existiert.
10. Als Entwickler möchte ich eine einzige Eigene-Konten-Config (IBAN +
    Konto-Rolle) als Quelle für `Gegenkonto::Eigen|Fremd` und die Rollen-Deutung,
    damit beides nicht auseinanderläuft.

## Implementation Decisions

### Architektur

- **Rust, Cargo-Workspace, hexagonal** (Ports & Adapter) — wartbare Software, kein
  Skript ([ADR-0001](../adr/0001-hexagonale-architektur.md),
  [ADR-0003](../adr/0003-rust-als-sprache.md)).
- **Domänen-Crate rein, kein I/O** (kein `reqwest`, kein `rusqlite`) und _kann_
  daher kein I/O; Adapter leben in eigenen Crates. `serde` ist erlaubt — aber nur
  auf obfuskierten DTOs; die Leak-Garantie ruht **nicht** auf der Abwesenheit von
  `serde`, sondern darauf, dass die Rohdaten-Newtypes (`Iban`, `Bic`) kein
  `Serialize` ableiten und in keinen serialisierbaren DTO einbettbar sind — ein
  Versuch ist ein Compile-Fehler ([ADR-0003](../adr/0003-rust-als-sprache.md)).

### Domänenmodell ([ADR-0005](../adr/0005-domaenenmodell-value-objects-und-summentypen.md))

- **Value Objects statt Primitiven** (_parse, don't validate_): `Betrag`
  (vorzeichenlos + Währung), `KontoId`, `Pseudonym`, `Fingerabdruck`,
  `GläubigerId`, `ObfuskierterText`. Geprüft wird einmal am Rand, danach trägt der
  Typ den Beweis.
- **Summentypen mit erschöpfendem `match`** statt Flag + optionale Felder:
  `Richtung::Eingang|Ausgang`, `Gegenkonto::Eigen|Fremd`,
  `Empfänger::Firma|Person`, `Buchungsart`. Ungültige Kombinationen sind **nicht
  konstruierbar**.
- `Betrag` ist vorzeichenlos; das Vorzeichen lebt ausschließlich in `Richtung`
  (−5 € mit `Eingang` nicht konstruierbar). Gemischte Währungen summiert die
  Domäne nicht — der eingehende Adapter sichert eine einheitliche Währung zu.
- Stil: **deklarativ-funktional** (reine Transformationen über unveränderliche
  Werte, Collection-Pipelines, `match`); Seiteneffekte leben am Rand.

### 4 Ports + Adapter ([ADR-0001](../adr/0001-hexagonale-architektur.md), [ADR-0002](../adr/0002-dto-grenze-und-anti-corruption-layer.md), [ADR-0004](../adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md))

- **`Umsatzquelle` (rein)** → CSV-CAMT-v8-Adapter zuerst. Obfuskiert beim Mappen,
  sodass die Domäne nur obfuskierte Value Objects erhält.
- **`Kategorisierer` (raus)** → spricht **eigene schmale DTOs**, nicht die
  `Buchung`-DTO: Hinweg `KategorieAnfrage` (Fingerabdruck + `ObfuskierterText` für
  Empfänger/Verwendungszweck), Rückweg `KategorieAntwort` (Fingerabdruck → Kategorie; schlüsselt auf dem
  Fingerabdruck, nie auf dem Text → keine Re-Identifikation). Chat-Payload-Adapter
  zuerst (JSON nur der unbekannten Fingerabdrücke → Chat → zurück), API-Adapter
  später, gleicher Port.
- **`Speicher` (raus)** → SQLite, speichert **DTOs** (nicht Domänenobjekte); das
  Mapping DTO ↔ Domäne lebt im Anti-Corruption-Layer, der das Kategorie-Register
  samt gelernter Korrekturen beim Laden rekonstituiert.
- **`Bericht` (raus)** → HTML zuerst; zeigt die Analyse der Domäne, die Kategorie
  ist nur ein Attribut einer `Buchung`.

### Domänen-Module (tief, am DTO-Vertrag testbar)

- **D1 Buchung-Kern** — die Value Objects + die Projektion
  `Buchung`→`KategorieAnfrage`.
- **D2 Fingerabdruck** — reine Funktion `Buchung`→`Fingerabdruck` inkl.
  Zahlungsintermediär-Erkennung + Händler-Token; Domänenwissen, **nicht** im
  Adapter ([ADR-0004](../adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)).
- **D3 ObfuskierterText** — Freitext-VO, der Scrubbing-Engpass; sein Konstruktor
  scrubbt best-effort, „ungescrubbt" ist gar nicht serialisierbar.
- **D4 Analyse-Kern** — Wiederkehr (inkl. Jahreskosten/Preissteigerung),
  Umbuchung-Netting (matcht auf `Gegenkonto::Eigen`; _welche_ Gegen-IBAN eigen
  ist, hat der Adapter schon aufgelöst), Gebühren-Klassifikation,
  Aderlass/Cashflow je Konto & Monat, Pareto-Ranking, Spar-Kandidaten.
- **D5 Kategorie-Register + gelernte Korrekturen** — schlüsselt auf dem
  Fingerabdruck.

### Lokale Konfiguration & Speicher (Adapter-seitig)

- **Eigene-Konten-Config** (lokal): je eigenem Konto IBAN + Konto-Rolle
  (Giro/Haushalt/Spar/Vorlauf). Eine Quelle für `Gegenkonto::Eigen|Fremd`
  (Schritt 2) **und** die Rollen-Deutung der Analyse (Schritt 3); die CSV trägt
  sie nicht. Verbindlich vor Schritt 2.
- **Zuordnungs-Speicher** (lokal, persistent, verlässt den Rechner nie):
  Klartext↔Pseudonym, getrennt vom obfuskierten `Speicher`-Port. First-Seen-
  Vergabe, einmal vergeben bleibt fest → laufstabile `PERSON_x`
  ([ADR-0006](../adr/0006-pseudonym-stabilitaet.md)). Verbindlich vor Schritt 4.
  Eigene Konten (`KONTO_x`) sind ohnehin stabil (aus der Eigene-Konten-Config).

### Datenschutz-Invarianten (zentral)

_Normative Fassung; die User Stories im Abschnitt „Datenschutz" spiegeln dieselben
Invarianten aus Nutzersicht._

- Den Rechner verlassen nur **obfuskierte, deduplizierte Fingerabdrücke** samt
  gescrubbtem Freitext.
- **Strukturelle Identifikatoren** (eigene IBAN/Kontonr., Gegen-IBAN, BIC) nie —
  **compilergeprüft** (`Iban`/`Bic` ohne `Serialize`, in keinem DTO).
- In Freitext **eingebettete** Identifikatoren werden best-effort gescrubbt
  (heuristisch, **nicht beweisbar vollständig**), fail-safe — die ehrliche Grenze
  des Modells.
- Privatnamen pseudonymisiert, Mapping bleibt lokal; `KategorieAntwort`
  schlüsselt auf dem Fingerabdruck → Re-Identifikation strukturell ausgeschlossen.
- Die Domäne erhält **keine** personen- oder kontobeziehbaren Identifikatoren —
  nur obfuskierte Value Objects; die **Gläubiger-ID** (öffentlicher
  Firmen-Schlüssel) bleibt bewusst als Analyse-Anker.
- **Obfuskierung lebt am eingehenden Adapter.**

### Baureihenfolge (Tracer-Bullet, vertikale Slices)

1. **Cargo-Workspace + Domänen-Skelett**: `Buchung` aus Value Objects (`Betrag`,
   `Richtung`, `Empfänger::Firma|Person`, `Gegenkonto::Eigen|Fremd`) plus der
   schmale `KategorieAnfrage`-Ausgangstyp (Freitext als `ObfuskierterText`). Test
   am DTO-Vertrag inkl. „Projektion `Buchung`→`KategorieAnfrage` trägt keinen
   strukturellen Identifikator". _(ersetzt das Python-Hello-World)_
2. **CSV-CAMT-v8-Adapter** gegen echte Beispiel-Datei → `Buchung`. Obfuskiert beim
   Mappen: IBAN/BIC bleiben im Adapter, `Auftragskonto`→`KONTO_x`,
   Privatperson→`PERSON_x`, Gegen-IBAN → `Gegenkonto::Eigen|Fremd` via
   Eigene-Konten-Config, Freitext → `ObfuskierterText` (best-effort gescrubbt);
   sichert eine einheitliche Währung zu.
3. **Analyse-Kern**: Wiederkehr + Gebühren + Aderlass + Pareto. Schon ohne LLM
   wertvoll (Lackmustest).
4. **`Speicher` (SQLite)** für DTOs + Kategorie-Register.
5. **`Kategorisierer`-Chat-Payload-Adapter** (Export → Chat → Import).
6. **HTML-`Bericht`**.
7. **Iteration**: gemeinsam durchgehen, Korrekturen lernen, Sparpotenziale
   benennen.

## Testing Decisions

### Was einen guten Test ausmacht

- **Black-Box über die DTO-Grenze**: Tests prüfen die Domäne gegen den
  öffentlichen DTO-Vertrag, ohne Implementierungsdetails zu kennen
  ([ADR-0002](../adr/0002-dto-grenze-und-anti-corruption-layer.md)). Das Innere
  bleibt frei refaktorierbar, solange der Vertrag hält.
- **TDD von der DTO-Grenze nach innen**: erst der Test am DTO-Vertrag, dann die
  Implementierung dahinter.
- **Organisation in vertikalen Slices** (agil): jeder Baureihenfolge-Schritt ist
  ein Slice; sein Test dockt an seiner DTO-Grenze an.
- **Konstruktor-Validierung wird mitgeprüft**: illegaler Wert → kein Objekt (kein
  „Objekt da, aber innen kaputt").

### Welche Module getestet werden

- **D1 Buchung-Kern + Projektion** (Schritt 1) ist der Start-Slice und der erste
  TDD-Test — mit dem tragenden Invariant-Test **„Projektion
  `Buchung`→`KategorieAnfrage` trägt keinen strukturellen Identifikator"**. Er
  wird zur Vorlage für die folgenden Slice-Tests.
- Jeder weitere Slice dockt nach **demselben Muster** an seiner DTO-Grenze an
  (D4 Analyse-Kern am DTO-Vertrag; Adapter an ihren schmalen DTOs), TDD von außen
  nach innen — nicht das Innenleben isoliert, sondern der Vertrag.

### Prior Art

- Das Repo ist frisch scaffolded — es gibt noch keinen Code und keine Tests. Prior
  Art ist daher der Vertrag selbst:
  [ADR-0002](../adr/0002-dto-grenze-und-anti-corruption-layer.md) (Black-Box am
  DTO-Vertrag) und
  [ADR-0005](../adr/0005-domaenenmodell-value-objects-und-summentypen.md) (Tests am
  DTO-Vertrag prüfen die Konstruktor-Validierung mit). Der erste Test aus Schritt 1
  setzt das Muster für alle weiteren.

## In Scope

Kompakt (Spiegel der README-„Drin"-Liste; Details in den User Stories): CSV-CAMT v8,
Mehrkonten, Umbuchung-Netting, Gebühren, Wiederkehr (inkl.
Jahreskosten/Preissteigerung), Kategorisierung via Chat, gelernte Korrekturen,
dynamischer Zeitraum (was die CSV hergibt), HTML-Report.

## Out of Scope

- **Bewusst draußen (YAGNI, später nur ein Adapter):** XML-camt, MT940,
  API-Kategorisierer, andere Report-Formate als HTML.
- **Keine Transaktionen/Kontoänderungen** — reine Analyse.
- **Keine Cloud-/Server-Verarbeitung**; kein Versand von Bankdaten an externe
  Dienste außer den obfuskierten, deduplizierten Fingerabdrücken (samt gescrubbtem
  Freitext) an das LLM-Orakel.
- **Beweisbar vollständiges Scrubbing** ist nicht im Scope — das Freitext-Scrubbing
  ist explizit heuristisch/best-effort.

## Further Notes

**Offene Punkte** (aus README „Offen" — vor ihren Schritten zu lösen; Details zu
Config und Zuordnungs-Speicher oben unter „Lokale Konfiguration & Speicher"):

- Echte **CSV-CAMT-v8-Beispieldatei** (ein Monat reicht) für Schritt 2 — Blocker
  für den Adapter.
- **Eigene-Konten-Config** (IBAN + Konto-Rolle) — verbindlich vor Schritt 2.
- **Persistente Pseudonym-Zuordnung** (laufstabile `PERSON_x`,
  [ADR-0006](../adr/0006-pseudonym-stabilitaet.md)) — verbindlich vor Schritt 4.
- **Scrubbing-Regeln** für `ObfuskierterText` (IBAN/BIC-Muster, bekannte
  Personen-Token), fail-safe — verbindlich vor Schritt 5.

**Veröffentlichung**: Es existiert kein Issue-Tracker (kein Remote/GitHub-Repo).
Dieser PRD liegt als lokaler Entwurf unter `docs/prd/`; Veröffentlichung in einen
Tracker später, falls gewünscht.

**Prozess**: Schritt 1 (Cargo-Workspace + Domänen-Skelett) startet erst auf Markus'
explizites „Go". Schritt 2 wartet auf die echte Beispiel-Datei.
