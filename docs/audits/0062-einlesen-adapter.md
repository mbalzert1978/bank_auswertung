---
id: "0062"
title: "Audit-Report — Gegenprüfung `einlesen_adapter` gegen die `.rules`"
status: done
bezug: "GitHub #61 / docs/issues/0062-gegenpruefung-einlesen-adapter-rules.md"
---

# Audit-Report — Gegenprüfung `einlesen_adapter` gegen die `.rules`

**Typ:** Reines Audit (kein Produktivcode geändert). Bezug:
[Issue #61](https://github.com/mbalzert1978/bank_auswertung/issues/61) /
[docs/issues/0062-gegenpruefung-einlesen-adapter-rules.md](../issues/0062-gegenpruefung-einlesen-adapter-rules.md).

**Redaktion:** Dieser Report nennt **keine** rohen Identifikatoren, IBANs, Klartext-Namen,
Beträge oder Seeds. Belege sind ausschließlich Datei-/Zeilen-Referenzen und statische
Bezeichner-/Typnamen.

## Geltungsbereich

Auditiert wurde die gesamte öffentliche und crate-private Fläche der Crate
`crates/einlesen_adapter`:

- Manifest: `Cargo.toml`
- Boundary/Fläche: `src/lib.rs`, `src/handler.rs`
- Modelle/DTOs: `src/models/{raw_record,line,ingested_message,read_command}.rs`
- Port-Bridges: `src/adapter/{source,store,ingested_publisher,calendar,money,currency}_bridge.rs`
- Öffentliche Seams: `src/abstractions/{source,store,publish,calendar,money,currency}_seam.rs`
- Test-Fläche: `src/testapi/read_test_api.rs`

Ergänzend gelesen zur Kontext-Bestätigung (nicht auditiert):
`crates/einlesen_domain/src/value_objects/raw_line.rs`.

## Ergebnis in einem Satz

**Alle einschlägigen Regeln sind konform.** Es wurde **keine** Abweichung gefunden; damit
**kein** Folge-Ticket zur Behebung. Ein bewusst **deferter**, dokumentierter Zustand (der
`Serialize`-Guard) wird wie im Ticket verlangt als erwartete offene Baustelle notiert — **kein
Verstoß**. Eine kleine, **optionale** Doku-Klarstellung (Wortwahl „Egress-Projektion") ist als
nicht-blockierender Vorschlag am Ende festgehalten.

## Befunde je Regel

### 1. `.rules/common/architektur.md`

#### 1a — Externe Information hinter einem Port — **konform**

Der Handler hängt ausschließlich an den von `einlesen_domain` verlangten Ports; die öffentlichen,
mechanischen Seams (`abstractions/`) sind roh-typisiert und domänen-blind, und die Port-Bridges
mappen dazwischen. Kein I/O-Crate wird im Adapter berührt.
Belege: `src/handler.rs:38-54` (Verdrahtung nur über Seams), `src/abstractions/mod.rs:1-4`
(Seams ohne Domänen-Kenntnis), `src/adapter/source_bridge.rs:29-44` (Bridge erfüllt den
Domänen-Port `RawLineSource`).

#### 1b — Die Domäne ist die Analyse; das LLM ist nur ein Orakel — **nicht einschlägig (konform)**

Der `einlesen`-Slice ist der Pipeline-Kopf (Lesen/Validieren/Erfassen), enthält keine Analyse
und ruft kein LLM/Orakel. Es gibt im Adapter keinen `Kategorisierer`-artigen Aufruf. Kein
Regelbezug verletzt.

#### 1c — Kopplung nur über die Outbox — kein synchroner Folgeaufruf — **konform**

Der Adapter publiziert die „Eingelesen"-Nachricht ausschließlich über den Domänen-Port
`EventSink`, den die Publisher-Bridge auf den öffentlichen `PublishSeam` mappt; **kein**
direkter Aufruf eines nachgelagerten Slice. Der Fan-out („ein `Ingested` pro gültiger Zeile")
wird vom Domänen-Handler getrieben — der Adapter stellt nur den Ein-Nachricht-Publish-Seam
bereit, ohne Folgeaufruf.
Belege: `src/adapter/ingested_publisher.rs:16-22` (Publish über den Seam, terminal gemappt),
`src/models/ingested_message.rs:35-41` (`PublishCommand` = reiner ID-Zeiger), `src/lib.rs:9-14`
(io fährt den Slice über den generischen Consumer).

#### 1d — Durchweg terminal — kein Retry, kein Dead-Letter im Slice — **konform**

Alle drei I/O-Schritte sind fehlbar und werden als reine `?`-/`map_err`-Kette auf den nackten,
redigierten Domänen-Fehler abgebildet; **keine** Retry-Schleife, **keine** Klassifikation im
Slice.
Belege: `src/adapter/source_bridge.rs:31-43` (Beschaffung + Präsenz, `?`-Kette), `src/adapter/
store_bridge.rs:19-24` (Ablage terminal → `SinkError::StoreFailed`), `src/adapter/
ingested_publisher.rs:17-21` (Publish terminal → `SinkError::PublishFailed`), `src/handler.rs:
57-76` (Handler gibt den nackten `ReadError` zurück).

#### 1e — Uniformes ID-Handoff — **konform**

Die Ablage-Bridge flacht das Aggregat auf das `Line`-DTO ab und lässt den Store die **neue ID**
vergeben (`store` gibt sie zurück); publiziert wird **nur** diese ID als Zeiger. Das ausgehende
Command bettet **kein** Fremd-Aggregat ein, sondern trägt allein die ID.
Belege: `src/adapter/store_bridge.rs:19-24` (Store vergibt die ID, in `IngestionId` gehüllt),
`src/abstractions/store_seam.rs:11-15` (Seam-Vertrag: Ablage vergibt die Identität),
`src/models/ingested_message.rs:22-41` (Message = nur `id`, `partition_key` = ID).

### 2. `.rules/common/privacy-invarianten.md`

#### 2a — Redigierte Fehler (statischer Feldname statt Rohwert) — **konform**

Der Präsenz-Fehler trägt **nur** den statischen Feldnamen (`&'static str`) — nie einen Rohwert;
auch `Debug` kann so keinen Identifikator spiegeln. Die Seam-Fehler (`SourceUnavailable`/
`StoreUnavailable`/`PublishUnavailable`) tragen laut Vertrag nur die io-Mechanik-Diagnose, nie
Satz-/Feldinhalt. Die Domänen-`Display` besitzt jeweils allein den Meldungssatz (kein Duplikat).
Belege: `src/models/raw_record.rs:63-74` (`MissingMandatoryField` opak, nur Feldname),
`src/adapter/source_bridge.rs:38-41` (Mapping auf `SourceError::MissingMandatoryField { field }`),
`src/abstractions/source_seam.rs:24-46` / `store_seam.rs:17-37` / `publish_seam.rs:16-35`
(Redaktions-Pflicht am Seam, `Display` reicht nur die Diagnose durch).

#### 2b — Status des Egress-`Serialize`-Guards — **konform (deferter Zustand, kein Verstoß)**

Wie vom Ticket verlangt festgehalten: Auf `Line` und `IngestedMessage` sitzt **kein**
`Serialize`-Guard, und die Crate bindet weder `serde` noch `static_assertions`. Das ist **kein**
Regelverstoß, sondern die dokumentierte, bewusste Entscheidung: `einlesen` ist **vor** der
Privacy-Grenze; rohe Werte dürfen **lokal** existieren (in-process über den Quell-Seam in die
Domäne, sowie in der unter der Aggregat-ID persistierten Roh-Erfassung). Die ausgehende Nachricht
trägt **nur die ID** — es findet **kein** Egress statt. Der Guard gehört daher auf die
Egress-DTOs der nachgelagerten Slices (`auswerten`/`report`).
Belege: `src/lib.rs:35-40` (explizite Begründung „bewusst kein `Serialize`-Guard"),
`src/models/line.rs:32-33`, `src/models/ingested_message.rs:16-20`.

**Zusätzliche Bestätigung (Ticket-Auftrag):** Die `Line`-Projektion leitet faktisch **keinen**
rohen Identifikator aus — sie wird nur **lokal** an den Speicher-Seam gereicht, und die Message
enthält ausschließlich die ID. Ein Compiler-Netz ist mangels `Serialize` auf allen Modell-Typen
gegeben (nichts kann serialisiert die Maschine verlassen). Belege: `src/models/line.rs:34-52`
(kein `#[derive(Serialize)]`), `src/adapter/store_bridge.rs:19-22` (Line nur an den Seam),
`src/models/ingested_message.rs:22-24` (Message = nur `id`).

#### 2c — Übrige Privacy-Invarianten — **nicht einschlägig (konform)**

„Freitext nur über `ObfuskierterText`", „Fail-safe: im Zweifel `Person`", „Fingerabdruck ist
reine Domänenfunktion", „Der Seed bleibt lokal" betreffen die Privacy-Grenze (`obfuskierung`) und
die Analyse (`auswerten`); der `einlesen`-Adapter kennt weder Pseudonym, Seed, Empfänger-Sumtyp
noch Fingerabdruck. Keiner dieser Mechanismen ist hier fehlplatziert.

### 3. `.rules/crates/dependencies-und-kapselung.md`

#### 3a — Nur `io` bindet konkrete Fremd-Crates — **konform**

`Cargo.toml` bindet ausschließlich `einlesen_domain` und `application_kernel`. **Kein**
I/O-Crate (`csv`/`rusqlite`/`chrono`/…); ein `grep` über `src/` findet **kein** `use` eines
Fremd-Crates.
Belege: `Cargo.toml` (`[dependencies]` = nur zwei path-Crates), verifiziert per Suche nach
`use csv|rusqlite|chrono|extern crate` (keine Treffer).

#### 3b — Fremd-Crate hinter einen Port — **konform**

Format-/Kalender-/Geld-/Währungs-Mechanik sitzt hinter den Seams (io-erfüllt); der Adapter
enthält **keine** konkrete Reader-/DB-Nutzung, nur Port-Bridges.
Belege: `src/abstractions/{calendar,money,currency,source,store,publish}_seam.rs` (Ports),
`src/adapter/*_bridge.rs` (nur Mapping).

#### 3c — Slice erreicht fremden Slice nur über dessen `_adapter` — **konform**

Der Adapter greift auf **keinen** fremden Slice zu — weder auf ein fremdes `_domain` noch auf ein
fremdes `_adapter`. Er bindet nur den eigenen `einlesen_domain` und den Kernel.
Beleg: `Cargo.toml` (keine fremde Slice-Dependency).

#### 3d — Nur `_adapter` bindet `application_kernel` — **konform**

`application_kernel` wird genau hier (im `_adapter`) gebunden und für den `Handler`-/`Command`-/
`CommandDecoder`-/`PublishCommand`-Vertrag genutzt.
Belege: `Cargo.toml`, `src/handler.rs:1` (`use application_kernel::Handler`),
`src/models/ingested_message.rs:1`, `src/models/read_command.rs:1`.

#### 3e — Ausnahme `serde`/`thiserror`/`static_assertions` — **konform (nicht genutzt)**

Die erlaubte Ausnahme wird gar nicht in Anspruch genommen (keine dieser Crates gebunden). Zulässig
und unauffällig.

### 4. `.rules/common/domaenenmodell.md`

#### 4 — Adapter-eigene Summentyp-DTOs statt re-exportierter Domänentypen — **konform**

`Direction`, `Sign`, `CalendarDay`, `ResolvedCurrency` sind **adapter-eigen** re-deklariert (kein
Re-Export der gleichnamigen Domänentypen); `Line` trägt die Währung als kanonischen ISO-Code-
`String`, nicht als Domänen-`Currency`. Value Objects (`NonEmptyString`) und Summentypen bleiben
domänenseitig; illegale Kombinationen sind unkonstruierbar (Präsenz lebt im Typ der Domäne).
Belege: `src/models/line.rs:9-22` (adapter-eigene `Direction` + `From`-Mapping),
`src/abstractions/money_seam.rs:38-42` (`Sign`), `src/abstractions/calendar_seam.rs:18-23`
(`CalendarDay`), `src/abstractions/currency_seam.rs:18-24` (`ResolvedCurrency`),
`crates/einlesen_domain/src/value_objects/raw_line.rs:15-26` (Pflichtfelder non-`Option`).

Hinweis: `src/lib.rs:66` re-exportiert `einlesen_domain::ReadError`. Das ist **kein** Verstoß
gegen diese Regel — der Fehler ist kein projizierter Datenwert-Summentyp, sondern der
Redaktions-Vertrag selbst (bewusst re-exportiert, damit die Domänen-`Display` alleinige
Eigentümerin des Meldungssatzes bleibt; dokumentiert in `src/lib.rs:60-66`).

### 5. `.rules/rust/idiome.md`

#### 5a — Newtypes validieren beim Konstruieren — **konform**

Die Präsenz-Abbildung reicht jede Zelle als `NonEmptyString`-Newtype (trimmt/weist Leeres ab) an
den Domänen-Rand; `MissingMandatoryField` ist ein opaker, redigierter Fehlertyp. Kein rohes
`String`/`f64` überquert die Domänengrenze.
Belege: `src/models/raw_record.rs:76-89` (`mandatory`/`as_nonempty` über `NonEmptyString::new`),
`src/models/raw_record.rs:63-74` (opaker Fehlertyp).

#### 5b — Leak = Compile-Fehler (Guard) — **konform (siehe 2b)**

Hier bewusst **kein** `assert_not_impl_any!`-Guard, weil `einlesen` vor der Privacy-Grenze liegt
und nichts ausleitet; dennoch besteht faktisch ein Compiler-Netz, da **kein** Modell-Typ
`Serialize` implementiert. Der Guard gehört auf die Egress-DTOs (`auswerten`/`report`). Keine
Abschwächung einer bestehenden Garantie. Beleg: `src/lib.rs:35-40`.

#### 5c — `<slice>_domain` ist dependency-frei — **nicht einschlägig für diese Crate**

Diese Regel adressiert die `_domain`-Crate; hier auditieren wir den `_adapter`. Der Adapter darf
den Kernel binden (tut er, siehe 3d). Kein Verstoß.

### 6. `.rules/rust/performance.md`

#### 6 — Zero-Cost-Iteratoren, nicht unnötig collecten — **konform**

Die einzige `collect`-Stelle sammelt in den vom Domänen-Port **geforderten** `Vec<RawLine>` und
short-circuitet über `Result` batch-atomar (kein überflüssiges Zwischen-`Vec`). Die
Ein-Währung-Prüfung läuft als reiner Iterator (`filter_map`/`all`, kein Zwischen-`Vec`); die
Betrags-Skalierung ist überlauf-geprüfte Ganzzahl-Mechanik ohne Allokation; die Projektion nutzt
`map`. Kein ungemessenes Klonen der Eingabe.
Belege: `src/adapter/source_bridge.rs:35-43` (nötiges `collect` in den Port-Rückgabetyp),
`src/adapter/source_bridge.rs:52-65` (`ensure_single_currency` iterator-basiert),
`src/adapter/money_bridge.rs:31-39` (`rescale` checked, allokationsfrei).

Anmerkung: Die tiefergehenden Regeln (Flamegraph/Heaptrack/Release-Profil) sind Werkzeug-/
Manifest-Regeln ohne Bezug zu diesem Bibliotheks-Crate-Code — nichts zu beanstanden.

### 7. `.rules/common/sprache.md`

#### 7 — Doku Deutsch, Infrastruktur-Bezeichner Englisch — **konform**

Sämtliche Doc-Kommentare sind auf Deutsch; Mechanik-/Infrastruktur-Bezeichner sind Englisch,
Domänen-Begriffe tragen ihre Fachsprache. Testnamen fallen nicht an (die Specs leben in
`einlesen_specs`, nicht in dieser Crate).
Belege: durchgängig, z. B. `src/lib.rs:1-40`, `src/adapter/source_bridge.rs:6-24`.

### 8. `.rules/rust/gates.md`

#### 8 — Alle sechs Gates grün — **konform**

Der Report fügt nur eine Markdown-Datei hinzu und ändert **keinen** Crate-Code; die sechs Gates
(`build`, `test`, `clippy --all-targets`, `fmt --check`, `cargo test -p architecture`,
`cargo doc --no-deps --workspace`) bleiben unverändert grün. Bestätigt über den Skill
`/cargo-six-gates` (PASS je Gate) im Rahmen dieses PR. Keine toten Doc-Links im auditierten
Code (Intra-Doc-Verweise zeigen auf existierende öffentliche Items; Kollaborateure werden nur
als Prosa/Code-Span genannt).

## Zusammenfassung

- **Geprüfte Regel-Dateien:** 8 (mit 20 atomaren Do/Don't-Prinzipien bzw. Sub-Befunden).
- **Konform:** alle. **Abweichungen:** 0. **Folge-Tickets zur Behebung:** 0.
- **Deferter, dokumentierter Zustand (kein Verstoß):** der `Serialize`-Guard auf den Egress-DTOs
  gehört bewusst nach `auswerten`/`report`, nicht in `einlesen` (bestätigt: keine faktische
  Ausleitung roher Identifikatoren; Compiler-Netz durch flächiges Fehlen von `Serialize`).

## Folge-Ticket-Vorschläge

Keine Behebungs-Tickets — es wurde keine Regelabweichung gefunden.

**Optionaler, nicht-blockierender Doku-Hinweis (kein Code-Change, keine Regelverletzung):**

- **Titel:** „Doku-Präzisierung: `Line` ist lokale Roh-Erfassung, nicht ‚Egress-Projektion'"
  **Betroffene Regel:** `.rules/common/privacy-invarianten.md` (Guard-Platzierung).
  **Ort:** Formulierung im Issue-Text 0062 bzw. ggf. Doc-Kommentar `src/models/line.rs:24-33`.
  **Grund:** Der Ticket-Text spricht von der „`Line`-Egress-Projektion". Code und `src/lib.rs:
  35-40` stellen jedoch klar, dass `Line` **keine** Egress-, sondern eine **lokal persistierte
  Roh-Erfassung** ist (die Message trägt nur die ID). Die aktuelle Doc-Formulierung ist bereits
  korrekt; einzig die Ticket-Wortwahl könnte den Eindruck einer Egress-Grenze erwecken. Reine
  Klarstellung, keine Verhaltens- oder Guard-Änderung nötig.
