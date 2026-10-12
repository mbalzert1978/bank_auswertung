# PRD: Bank-Auswertung

> **Entwurf (lokal).** Dieser PRD liegt als lokaler Entwurf unter `docs/prd/`.
> Inzwischen existiert ein GitHub-Repo
> ([mbalzert1978/bank_auswertung](https://github.com/mbalzert1978/bank_auswertung))
> mit Issue-Tracker; laufende Arbeit ist in [docs/issues/](../issues/) (samt
> [PROGRESS.md](../issues/PROGRESS.md)) verortet.
>
> Quelle der Wahrheit für **Zweck, Umfang und Datenschutz**:
> [README.md](../../README.md); für die **Architektur**: [docs/adr/](../adr/) (die README
> trägt bewusst keine Architekturdetails mehr). Begriffe:
> [CONTEXT.md](../../CONTEXT.md). Dieser PRD erfindet nichts über README und ADRs hinaus.
>
> **Architektur superseded durch
> [ADR-0007](../adr/0007-modularer-monolith-feature-slices.md):** Die Abschnitte zu
> Cargo-Workspace/„reine Domänen-Crate" und die Domänen-Module **D1–D5 in einem
> Crate** sind überholt — Ziel ist ein modularer Monolith aus Feature-Slices
> (Triade `_domain`/`_adapter`/`_specs`, ein dünner interface-only Kernel, kein geteiltes
> `domain`-Crate; dieser Kernel heißt heute **`application_kernel`** — s. u.).
> Fachfunktionen, Datenschutz-Invarianten und Baureihenfolge bleiben gültig; nur ihre
> Verortung ändert sich.
>
> **Maßgeblich ist jetzt
> [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):** eine
> **event-getriebene Pipeline aus Slices mit Outbox** — seit
> [ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md) (akzeptiert)
> `einlesen → obfuskieren → fenstern → auswerten → report` —, die ADR-0007 in Teilen
> verfeinert/supersedet. Sie ändert
> die **Ablauf-Verortung** — jeder Schritt ist ein eigener Consumer-Slice, gekoppelt
> nur über Outbox-Messages — und **reframt die Datenschutz-Grenze** von „`Iban`/`Bic`
> sind nie serialisierbar" auf „roh verlässt die Maschine nie" (Grenze an
> `obfuskieren`, s. u.). Die Fachfunktionen und das Datenschutz-*Ziel* bleiben
> unverändert. Supersession-Index über alle ADRs: [docs/adr/README.md](../adr/).

## Problemstellung

Aus Markus' Sicht: Monatliche Kosten ziehen seine Konten leer, und er sieht nicht
klar, wohin. Über mehrere Konten hinweg verschwimmt, was echter Abfluss ist und was
nur eine Umbuchung zwischen den eigenen Konten ist. Er will zweierlei: (1) wissen,
**wo das Geld wegblutet** — die Hauptkostentreiber —, und (2) **realistische
Sparpotenziale** finden. Gleichzeitig
sind das sensible Bankdaten: Er will sie lokal verarbeiten und nichts
Identifizierendes an externe Dienste geben.

## Lösung

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

## Nutzergeschichten

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

### Bericht

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
    Konto-Rolle) als Quelle für `Gegenkonto::Eigen(Rolle)|Fremd` und die Rollen-Deutung,
    damit beides nicht auseinanderläuft.

## Umsetzungsentscheidungen

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
  > **Datenschutz-Verortung superseded durch
  > [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):** die
  > „kein `Serialize`"-Sperre sitzt nicht mehr auf `Iban`/`Bic`, sondern auf den
  > **Egress-DTOs**. Details unten unter „Datenschutz-Invarianten". Das Ziel (kein
  > realer Identifikator verlässt den Rechner) bleibt unverändert.

### Event-getriebene Pipeline aus Slices ([ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md))

Diese Sektion ist der aktuelle Stand der Architektur-Verortung; die Verteilung der
Domänen-Module auf Crates aus „Domänen-Module (tief, …)" ist entsprechend
verschoben (s. dort den Supersession-Hinweis).

- **Pipeline `einlesen → obfuskieren → fenstern → auswerten → report`.** Jeder Schritt ist ein
  eigener **Consumer-Slice** (Triade `_domain`/`_adapter`/`_specs`), der **ein**
  eingehendes Command verarbeitet und in **einer** Transaktion (`UnitOfWork`) (a)
  seine eigene Tabelle und (b) eine **Outbox-Message** für den nächsten Schritt
  schreibt (Outbox-Pattern → atomar, at-least-once, wiederaufsetzbar). Kein Schritt
  ruft den nächsten synchron; die Kopplung läuft **nur** über die Message.
  > **Verfeinert durch [ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)
  > (akzeptiert):** zwischen `obfuskieren` und `auswerten` sitzt der Schritt **`fenstern`** —
  > er paginiert die obfuskierte Historie und materialisiert je Seite ein
  > **`Auswertungsfenster`**-Aggregat (N→1), dessen ID er publiziert. `auswerten` spaltet in
  > **zwei Reichweiten** (pro Fenster / fenster-übergreifend); Begriffe in
  > [CONTEXT.md](../../CONTEXT.md).
- **Kategorisieren ist kein Pipeline-Schritt**, sondern eine Domänen-Entscheidung
  im `auswerten`-Slice: entscheidet die Domäne, dass ein Fingerabdruck unbekannt
  ist, ruft der Handler den injizierten `Kategorisierer`-Port (LLM als Orakel). Die
  bewusste Asymmetrie zu `obfuskieren` (mechanische, immer nötige Grenze = eigener
  Schritt) folgt [ADR-0004](../adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md).
- **Rollen je Slice:** der **Handler** (crate-intern) orchestriert das reiche
  Aggregat über **Domänen-Typen** (assoziierte `Command`/`Response`/`Error`, **kein**
  public IO); der **Adapter** ist die Anpassung an die Außenwelt — **Consume-Bridge**
  (`OutboxHandlerBase`-Muster: Message → Domänen-Command → Handler → `HandleOutcome`),
  **Publisher-Bridge** (Domänen-Command → Wire-DTO `OutboxCommand` → Outbox) und
  **Port-Bridges** (implementieren die Domänen-Ports via public Seams); die **Domäne**
  bleibt blind und kennt keinen Adapter, keinen fremden Slice, keine Wire-Form.
- **`shared_kernel`** bleibt dünn und interface-only (nur `std`), trägt aber die
  **Messaging-/Outbox-Verträge**: `Handler`/`Command`/`Publisher` sowie
  `OutboxMessage`/`OutboxCommand`/`OutboxPublisher`/`OutboxHandler`/`HandleOutcome`/
  `Severity`/`UnitOfWork` (+ JSON-Konvention). Die blinde Domäne nutzt sie nie.
  > **Verfeinert durch [ADR-0009](../adr/0009-kernel-split-und-idiomatische-outbox-vertraege.md):**
  > Der eine `shared_kernel` ist in **zwei** Crates geschnitten — `shared_kernel`
  > (io-facing Outbox-Seams) + `application_kernel` (slice-interne
  > `Handler`/`Command`/`Publisher`/`TypedOutboxHandler`) — weil Rust kein cross-crate
  > `internal` kennt, sodass `io` die slice-internen Verträge nicht sieht. Die
  > Vertragsnamen oben (auch das `OutboxHandlerBase`/`HandleOutcome` aus „Rollen je
  > Slice") sind Rust-idiomatisch: `Result<(), OutboxError>` statt
  > `HandleOutcome`/`Severity`, `publish` statt `add_to_outbox`, `const MESSAGE_TYPE`,
  > `TypedOutboxHandler` statt `OutboxHandlerBase`.
  > **Erneut verfeinert durch [ADR-0011](../adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
  > (gebaut):** der Zwei-Crate-Schnitt ist wieder **zusammengeführt** — die Crate
  > `shared_kernel` wurde **aufgelöst** (Issue 0032), ihre Outbox-Seams liegen jetzt
  > **`io`-privat** in `io`. **`application_kernel` ist der einzige** Slice↔`io`-Vertrag
  > (`Command`/`Handler`/`Publisher`/`PublishCommand`/`CommandDecoder`); der
  > `<slice>_adapter` bindet nur noch ihn und nennt keinen Outbox-Typ mehr.
- **Testbar jetzt, echtes IO später:** jeder Slice fährt als Black-Box über seine
  Fluent-TestApi mit **Fakes** (Fake-`OutboxPublisher`, Fake-Stores, Fake-Quelle);
  das echte `io`-Crate (SQLite-Outbox + -Stores, Datei/CSV-Parse) erfüllt später
  dieselben Seams.
  > **Überholt** ([ADR-0014](../adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md),
  > Issues 0050–0059, 0083): **alle vier** Slices haben ihren realen `io`-Store und ihre
  > reale Lese-Quelle; das Programm `bank-auswertung` fährt die ganze Kette gegen echtes
  > SQLite und die echte CSV-Quelle. Die Fluent-TestApis der Slices laufen weiter gegen
  > Fakes — das ist die Black-Box-Spezifikation, nicht ein fehlender Adapter.
- **Referenz:** die interne C#-Vorlage unter `examples/` (Slices `Downloader`/
  `Stager`, `Outbox`, `Common`) ist das kanonische Vorbild für Handler, Bridges und
  Outbox-Verträge.

### Domänenmodell ([ADR-0005](../adr/0005-domaenenmodell-value-objects-und-summentypen.md))

- **Value Objects statt Primitiven** (_parse, don't validate_): `Betrag`
  (vorzeichenlos + Währung), `KontoId`, `Pseudonym`, `Fingerabdruck`,
  `GläubigerId`, `ObfuskierterText`. Geprüft wird einmal am Rand, danach trägt der
  Typ den Beweis.
- **Summentypen mit erschöpfendem `match`** statt Flag + optionale Felder:
  `Richtung::Eingang|Ausgang`, `Gegenkonto::Eigen(Rolle)|Fremd`,
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
  > **Superseded durch [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):**
  > `obfuskieren` ist **kein** Inline-Schritt des Einlese-Adapters mehr, sondern ein
  > **nachgelagerter Consumer-Slice** in der Pipeline (`einlesen → obfuskieren`); die
  > Kopplung ist die Outbox-Message, nicht ein synchroner Bridge-Aufruf. Der
  > `einlesen`-Slice darf roh lokal erfassen; erst `obfuskieren` ist die Grenze.
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

> **Verortung superseded durch [ADR-0007](../adr/0007-modularer-monolith-feature-slices.md)
> / [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):** die Module
> liegen **nicht mehr in einem Domänen-Crate**, sondern verteilt auf die
> Pipeline-Slices — Obfuskierung (D3) im `obfuskieren`-Slice, Fingerabdruck (D2),
> Analyse-Kern (D4) und Kategorie-Register (D5) im `auswerten`-Slice, der
> Buchung-Kern (D1) im einlesenden/`auswerten`-Slice. Die fachliche Aufteilung und
> die Testbarkeit am DTO-Vertrag bleiben; nur die Crate-Verortung ändert sich.

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
  (Giro/Haushalt/Spar/Vorlauf). Eine Quelle für `Gegenkonto::Eigen(Rolle)|Fremd`
  (Schritt 2) **und** die Rollen-Deutung der Analyse (Schritt 3); die CSV trägt
  sie nicht. Verbindlich vor Schritt 2.
- **Zuordnungs-Speicher** (lokal, persistent, verlässt den Rechner nie):
  Klartext↔Pseudonym, getrennt vom obfuskierten `Speicher`-Port. First-Seen-
  Vergabe, einmal vergeben bleibt fest → laufstabile `PERSON_x`
  ([ADR-0006](../adr/0006-pseudonym-stabilitaet.md)). Verbindlich vor Schritt 4.
  Eigene Konten (`KONTO_x`) sind ohnehin stabil (aus der Eigene-Konten-Config).
  > **Superseded durch [ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md):**
  > **kein** persistenter Zuordnungs-Speicher und **keine** First-Seen-Vergabe mehr. Die
  > Pseudonym-Stabilität ruht auf einem **config-injizierten Seed** (deterministische Token
  > aus `(Seed, Klartext)`, mitpersistiert). Das lokal zu haltende Geheimnis ist der
  > **Seed**; laufstabile `PERSON_x`, solange die Config denselben Seed liefert. `KONTO_x`
  > bleibt aus der Eigene-Konten-Config stabil.

### Datenschutz-Invarianten (zentral)

_Normative Fassung; die User Stories im Abschnitt „Datenschutz" spiegeln dieselben
Invarianten aus Nutzersicht._

- Den Rechner verlassen nur **obfuskierte, deduplizierte Fingerabdrücke** samt
  gescrubbtem Freitext.
- **Strukturelle Identifikatoren** (eigene IBAN/Kontonr., Gegen-IBAN, BIC) nie —
  **compilergeprüft** (`Iban`/`Bic` ohne `Serialize`, in keinem DTO).
  > **Superseded durch [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):**
  > nicht mehr „roh ist nie serialisierbar", sondern **„roh verlässt die Maschine
  > nie"**. Roh darf **lokal existieren und persistieren** (die `einlesen`-Tabelle und
  > die `einlesen → obfuskieren`-Message tragen rohe Werte — kein Unterschied zur
  > lokal liegenden `.csv`). Die compilergeprüfte **`assert_not_impl_any!`-„kein
  > `Serialize`"-Sperre wandert von `Iban`/`Bic` auf die Egress-DTOs** (LLM-Anfrage
  > und Report-Output) — genau die Punkte, an denen Daten die Maschine verlassen.
  > `Iban` **muss** für den lokalen `einlesen → obfuskieren`-Hop serialisierbar sein.
  > Das Ziel (kein realer Name/Identifikator zum LLM/Report) bleibt unverändert.
- In Freitext **eingebettete** Identifikatoren werden best-effort gescrubbt
  (heuristisch, **nicht beweisbar vollständig**), fail-safe — die ehrliche Grenze
  des Modells.
- Privatnamen pseudonymisiert, Mapping bleibt lokal; `KategorieAntwort`
  schlüsselt auf dem Fingerabdruck → Re-Identifikation strukturell ausgeschlossen.
- Die Domäne erhält **keine** personen- oder kontobeziehbaren Identifikatoren —
  nur obfuskierte Value Objects; die **Gläubiger-ID** (öffentlicher
  Firmen-Schlüssel) bleibt bewusst als Analyse-Anker.
- **Obfuskierung lebt am eingehenden Adapter.**
  > **Superseded durch [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):**
  > **`obfuskieren` ist die Datenschutz-Grenze** und ein **eigener nachgelagerter
  > Pipeline-Schritt** hinter `einlesen`, kein Inline-Teil des Einlese-Adapters. Ab
  > `obfuskieren` laufen nur noch Token.

### Baureihenfolge (Tracer-Bullet, vertikale Slices)

> **Überholt** durch [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md)/
> [ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)/
> [ADR-0014](../adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md): die Liste unten ist
> die ursprüngliche Planung vor der event-getriebenen Pipeline. Die aktuelle Baureihenfolge
> und der reale Stand stehen im Status-Abschnitt und unter „Build order" in
> [CLAUDE.md](../../CLAUDE.md).

1. **Cargo-Workspace + Domänen-Skelett**: `Buchung` aus Value Objects (`Betrag`,
   `Richtung`, `Empfänger::Firma|Person`, `Gegenkonto::Eigen(Rolle)|Fremd`) plus der
   schmale `KategorieAnfrage`-Ausgangstyp (Freitext als `ObfuskierterText`). Test
   am DTO-Vertrag inkl. „Projektion `Buchung`→`KategorieAnfrage` trägt keinen
   strukturellen Identifikator". _(ersetzt das Python-Hello-World)_
2. **CSV-CAMT-v8-Adapter** gegen echte Beispiel-Datei → `Buchung`. Obfuskiert beim
   Mappen: IBAN/BIC bleiben im Adapter, `Auftragskonto`→`KONTO_x`,
   Privatperson→`PERSON_x`, Gegen-IBAN → `Gegenkonto::Eigen(Rolle)|Fremd` via
   Eigene-Konten-Config, Freitext → `ObfuskierterText` (best-effort gescrubbt);
   sichert eine einheitliche Währung zu.
   > **Superseded durch [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md):**
   > gesplittet in zwei Pipeline-Schritte — **`einlesen`** (Rohdaten prüfen/erfassen,
   > CSV-CAMT-v8 ist nur eine Eingabeform hinter dem Quell-Seam) und das nachgelagerte
   > **`obfuskieren`** (Token-Vergabe + Freitext-Scrubbing). Die **Slice-Umbenennung
   > `csv_camt_v8` → `einlesen`** ist **ausgeführt** (Crates `einlesen_*`).
3. **Analyse-Kern**: Wiederkehr + Gebühren + Aderlass + Pareto. Schon ohne LLM
   wertvoll (Lackmustest).
4. **`Speicher` (SQLite)** für DTOs + Kategorie-Register.
5. **`Kategorisierer`-Chat-Payload-Adapter** (Export → Chat → Import).
6. **HTML-`Bericht`**.
7. **Iteration**: gemeinsam durchgehen, Korrekturen lernen, Sparpotenziale
   benennen.

## Testentscheidungen

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

### Vorarbeiten

- Das Repo ist frisch scaffolded — es gibt noch keinen Code und keine Tests. Prior
  Art ist daher der Vertrag selbst:
  [ADR-0002](../adr/0002-dto-grenze-und-anti-corruption-layer.md) (Black-Box am
  DTO-Vertrag) und
  [ADR-0005](../adr/0005-domaenenmodell-value-objects-und-summentypen.md) (Tests am
  DTO-Vertrag prüfen die Konstruktor-Validierung mit). Der erste Test aus Schritt 1
  setzt das Muster für alle weiteren.

## Im Umfang

Kompakt (Spiegel der README-„Drin"-Liste; Details in den User Stories): CSV-CAMT v8,
Mehrkonten, Umbuchung-Netting, Gebühren, Wiederkehr (inkl.
Jahreskosten/Preissteigerung), Kategorisierung via Chat, gelernte Korrekturen,
dynamischer Zeitraum (was die CSV hergibt), HTML-Report.

## Außerhalb des Umfangs

- **Bewusst draußen (YAGNI, später nur ein Adapter):** XML-camt, MT940,
  API-Kategorisierer, andere Report-Formate als HTML.
- **Keine Transaktionen/Kontoänderungen** — reine Analyse.
- **Keine Cloud-/Server-Verarbeitung**; kein Versand von Bankdaten an externe
  Dienste außer den obfuskierten, deduplizierten Fingerabdrücken (samt gescrubbtem
  Freitext) an das LLM-Orakel.
- **Beweisbar vollständiges Scrubbing** ist nicht im Scope — das Freitext-Scrubbing
  ist explizit heuristisch/best-effort.

## Weitere Hinweise

**Offene Punkte** (aus README „Offen" — vor ihren Schritten zu lösen; Details zu
Config und Zuordnungs-Speicher oben unter „Lokale Konfiguration & Speicher"):

- Echte **CSV-CAMT-v8-Beispieldatei** (ein Monat reicht) für Schritt 2 — Blocker
  für den Adapter.
- **Eigene-Konten-Config** (IBAN + Konto-Rolle) — verbindlich vor Schritt 2.
- **Stabiler Config-Seed** (laufstabile `PERSON_x` seed-basiert,
  [ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md); ersetzt die frühere
  persistente Pseudonym-Zuordnung aus ADR-0006) — verbindlich vor einem echten
  `obfuskieren`-Lauf.
- **Scrubbing-Regeln** für `ObfuskierterText` (IBAN/BIC-Muster, bekannte
  Personen-Token), fail-safe — verbindlich vor Schritt 5.

**Veröffentlichung**: Dieser PRD liegt als lokaler Entwurf unter `docs/prd/` (siehe
Hinweis zum GitHub-Repo + Issue-Tracker im Kopf dieses Dokuments).

**Prozess**: Schritt 1 (Cargo-Workspace + Domänen-Skelett) startet erst auf Markus'
explizites „Go". Schritt 2 wartet auf die echte Beispiel-Datei.
> **Überholt:** Der Bau ist seither weit über Schritt 1/2 hinaus fortgeschritten (die
> `Baureihenfolge` oben ist die ursprüngliche Planung vor ADR-0008/0013/0014);
> aktueller Stand + Baureihenfolge stehen im Status-Abschnitt von
> [CLAUDE.md](../../CLAUDE.md).
