---
id: "0061"
title: "Gegenprüfung `einlesen_domain` gegen die `.rules`"
typ: audit
status: abgeschlossen
bezug: "GitHub #60"
crate: crates/einlesen_domain
---

# Prüf-Report — Gegenprüfung `einlesen_domain` gegen die `.rules`

**Reines Audit.** Es wurde **keine Zeile Produktivcode** geändert; einziger Zusatz dieses
PR ist dieser Report. Jede im Ticket ([#60](https://github.com/mbalzert1978/bank_auswertung/issues/60))
gelistete `.rules`-Regel ist gegen den Crate-Code geprüft; je Regel genau **ein Befund**
(konform / Abweichung mit `datei:zeile`). Der Report ist **redigiert**: er nennt keine rohen
Identifikatoren, IBANs, Klartext-Namen oder Seeds, nur Datei- und Zeilenverweise.

Geprüfter Stand: Branch `audit/0061-einlesen-domain` (von `main`). Geprüfte Dateien: der
gesamte Baum unter `crates/einlesen_domain/src/` plus `crates/einlesen_domain/Cargo.toml`.

## Zusammenfassung

- **20 Regel-Befunde** insgesamt: **17 konform**, **2 bewusst nicht einschlägig** (NA, im
  Ticket ausdrücklich ausgeklammert), **1 weiche Abweichung** (fehlender `proptest`-Einsatz).
- **Keine harte Architektur- oder Dependency-Abweichung.** Die Domäne ist dependency-frei,
  terminal, port-basiert und deklarativ-funktional.
- **Drei Folge-Ticket-Vorschläge** (einer aus der weichen Abweichung, zwei aus
  Beobachtungen niedriger Priorität) — siehe Abschnitt am Ende.

## Befunde je Regel

### `.rules/common/domaenenmodell.md`

1. **Value Objects statt Primitiven — konform.** Kein rohes `String`/`f64` überschreitet
   die Domänen-Grenze: der Adapter reicht `NonEmptyString`
   (`value_objects/non_empty_string.rs:7`) herein, aus dem sich alle Wert-Typen bauen
   (`Iban`, `Purpose`, `BookingText`, `CreditorId`, `RecipientName`,
   `value_objects/raw_line.rs:15`). `Amount` (`value_objects/amount.rs:50`),
   `Currency` (`value_objects/amount.rs:13`), `CalendarDay`/`Date` (`value_objects/date.rs:9`,
   `:46`) sind Typen, die geprüfte Bestandteile bündeln. Dass die Getter von `CalendarDay`
   Zahlen (`u16`/`u8`) herausgeben (`value_objects/date.rs:26–38`), ist bewusst und
   dokumentiert (Rhythmus-Erkennung im `auswerten`-Slice), kein Rohmaterial-Eingang.

2. **Summentypen statt Flag + optionale Felder — konform.** `Direction` und `Sign` sind
   Enums (`value_objects/direction.rs:8`, `:18`), kein `bool`-Flag. `Recipient` trägt zwei
   `Option`-Felder (`value_objects/recipient.rs:60–64`); das ist **kein** verstecktes
   Zustands-Flag, sondern genuin optionale Datenfelder — die Firma/Person-Deutung ist laut
   Dokumentation (`value_objects/recipient.rs:51–59`) bewusst in den nachgelagerten
   `obfuskieren`-Slice verlagert, weil sie heuristisch/fail-safe ist. Der Summentyp entsteht
   dort, nicht hier. Die Modellierung ist mit der Regel vereinbar.

3. **`Betrag` vorzeichenlos — konform.** `Amount` hält `minor_units: u64`
   (`value_objects/amount.rs:51`), vorzeichenlos; das Vorzeichen lebt allein in `Direction`
   (`value_objects/direction.rs:18`, `from_sign` unter `:28`). Ein negativer Betrag ist
   unkonstruierbar. Gemischte Währungen wehrt die ACL vorab ab (`error.rs:70`,
   `SourceError::MixedCurrency`), die Domäne rechnet nicht über gemischte Währungen.

4. **Deklarativ-funktional — konform.** Der Handler ist eine reine Iterator-Pipeline
   (`handler.rs:52–65`): `fetch → map(from_raw_line) → collect::<Result<Vec<_>,_>>()? →
   try_for_each(store+publish)`. Keine manuelle `mut`-Schleife im Crate; die Verzweigungen
   sind erschöpfende `match` (`direction.rs:29–32`, `error.rs` Display-`match`).

### `.rules/rust/idiome.md`

5. **`<slice>_domain` dependency-frei — konform.** `Cargo.toml` führt einen **leeren**
   `[dependencies]`-Block (`crates/einlesen_domain/Cargo.toml:9`); kein `serde`, kein
   `thiserror`, kein I/O-Crate. Verifiziert: kein `use serde`/`thiserror`/`chrono`/`csv`/
   `rusqlite` im gesamten `src/`-Baum. Alle Domänen-Fehler tragen **handgeschriebenes**
   `Display` + `std::error::Error` (`error.rs:32–43`, `:73–87`, `:105–114`, `:130–151`).

6. **Leak = Compile-Fehler (`Serialize`-Guard) — nicht einschlägig (NA).** Der
   `static_assertions`-„kein `Serialize`"-Guard sitzt auf **Egress-DTOs**; die Domäne hat
   keine und bindet kein serde. Das Ticket klammert die Serialize-Guard-Regel für diese
   dependency-freie Domäne ausdrücklich aus. Konsistent umgesetzt: `Iban`
   (`value_objects/iban.rs:4–8`) dokumentiert explizit, dass hier bewusst **kein**
   Serialize-Guard nötig ist (rohe Werte reisen nur lokal, kein Egress).

7. **Newtypes validieren beim Konstruieren — konform (mit einer Beobachtung).**
   `NonEmptyString::new` trimmt und weist Leeres als `None` ab
   (`value_objects/non_empty_string.rs:13–20`); `RawLine`-Pflichtfelder sind non-`Option`
   (`value_objects/raw_line.rs:16–20`) — Präsenz lebt im Typ, nicht im Laufzeit-Check.
   **Beobachtung (kein Regelbruch):** `IngestionId::new` validiert nicht per `Option`/`Result`,
   sondern **panickt** bei leerem Rohwert via `assert!`
   (`value_objects/ingestion_id.rs:24–32`). Das ist dokumentiert und begründet (io garantiert
   eine nie-leere ID; ein Leerwert wäre ein Herkunfts-Invariantenbruch, kein deutbarer
   Eingang). Es ist damit ein bewusst gewählter Panic-Pfad in der Domäne, der von der
   „Don't Panic"-Clippy-Regel **nicht** erfasst wird (siehe Befund 20). → Folge-Ticket A.

### `.rules/common/architektur.md`

8. **Externe Information hinter einem Port — konform.** Jede externe Information sitzt hinter
   einem von der Domäne verlangten Trait: `RawLineSource` (`ports/raw_line_source.rs:14`),
   `Calendar` (`ports/calendar.rs:12`), `Money` (`ports/money.rs:13`), `Currencies`
   (`ports/currency.rs:12`), `Store` (`ports/store.rs:14`), `EventSink`
   (`ports/event_sink.rs:13`). Kein Format-/Anbieter-/Speicher-Wissen in der Domäne.

9. **Die Domäne ist die Analyse; das LLM ist nur ein Orakel — nicht einschlägig (NA).**
   `einlesen` ist der Pipeline-Kopf (lesen/validieren/erfassen); es rechnet keine Analyse und
   ruft kein LLM/Orakel. Die Regel greift im `auswerten`-Slice, nicht hier.

10. **Kopplung nur über die Outbox — konform.** Der Handler veröffentlicht die ID über den
    `EventSink`-Port (`handler.rs:63`) statt einen Folge-Slice synchron aufzurufen. Kein
    direkter Aufruf eines fremden Slice im Crate.

11. **Durchweg terminal — kein Retry/Dead-Letter — konform.** Alle drei I/O-Schritte sind
    fehlbar und werden per reiner `?`-Kette propagiert (`handler.rs:52–65`); keine
    Retry-Schleife, keine Fehler-Klassifikation. Der Handler gibt seinen nackten,
    redigierten `ReadError` zurück (`error.rs:120–128`). Die Redaktion ist per Konstruktion
    abgesichert: keine Fehler-Variante trägt einen rohen Feldinhalt (`error.rs:19–30`,
    `:60–71`); die durchgereichten `reason`-Strings sind Zugriffs-/io-Diagnose, am Seam als
    redaktionspflichtig dokumentiert (`error.rs:56–59`, `:94–96`).

12. **Uniformes ID-Handoff — konform.** Der `Store` vergibt die `IngestionId` beim Ablegen
    (`ports/store.rs:17`, `handler.rs:62`), und **genau diese** neue ID wird als Zeiger
    veröffentlicht (`handler.rs:63`). Als Pipeline-Kopf liest `einlesen` keinen Vorgänger per
    ID (dieser Teil der Regel ist strukturell NA); das Aggregat bettet kein Fremd-Aggregat
    ein (`entities/line_aggregate.rs:14–23` trägt nur eigene Wert-Objekte).

### `.rules/crates/dependencies-und-kapselung.md`

13. **Nur `io` bindet konkrete Fremd-Crates — konform.** `[dependencies]` ist leer
    (`crates/einlesen_domain/Cargo.toml:9`); kein I/O-Crate im Slice.

14. **Fremd-Crate hinter einen Port — konform.** Kein `use <fremd_crate>` im `src/`-Baum;
    jede externe Fähigkeit ist als Port abstrahiert (siehe Befund 8).

15. **Slice erreicht fremden Slice nur über dessen `_adapter` — konform.** Kein `use` eines
    fremden `_domain`/`_adapter` im Crate; die Domäne ist blind für andere Slices.

16. **Nur `_adapter` bindet `application_kernel` — konform.** `einlesen_domain` bindet
    `application_kernel` **nicht** (weder in `Cargo.toml` noch per `use`).

### `.rules/rust/performance.md`

17. **Zero-Cost-Iteratoren, keine unnötigen `collect` — konform.** Der eine `collect` im
    Crate (`handler.rs:58`, `collect::<Result<Vec<_>,_>>()`) ist **semantisch erforderlich**,
    nicht überflüssig: erst wenn **alle** Zeilen erfolgreich zu Aggregaten gedeutet sind,
    darf die erste Ablage passieren — so hinterlässt eine kaputte Zeile nichts Halbes
    (dokumentiert `handler.rs:9–14`). Ein Kürzen zu einem einzigen Stream würde die
    „erst validieren, dann ablegen"-Semantik brechen. Die Mess-/Profiling-Werkzeuge der
    Regel (`cargo-flamegraph`/`cargo-heaptrack`, Release-Profil) betreffen den io-/Binär-Rand,
    nicht diese dependency-freie Domäne.

### `.rules/rust/testing-werkzeuge.md`

18. **`proptest` für Aggregat-Invarianten — weiche Abweichung.** Der Crate enthält **keinen**
    `proptest`; die Invarianten werden über Beispiel-Tests abgedeckt: inline in
    `value_objects/non_empty_string.rs:28–43` (Trim/Leere) und
    `value_objects/ingestion_id.rs:40–57` (Panic/Durchreichen), das Aggregat-Verhalten
    transitiv über die Black-Box-Specs (`crates/einlesen_specs/`). Die Regel ist ausdrücklich
    „soweit vorhanden/sinnvoll" formuliert; es ist daher eine **weiche** Abweichung, kein
    harter Bruch. Property-Tests für die zwei einfachen, gut kapselbaren Invarianten
    (`NonEmptyString`-Trim-Idempotenz; vorzeichenloser `Amount` über den Eingaberaum des
    `DecodedAmount`) wären ein sinnvoller, kleiner Zugewinn. → Folge-Ticket B.

### `.rules/common/sprache.md`

19. **Docs/Tests Deutsch, Infrastruktur-Code Englisch — konform (mit einer Beobachtung).**
    Alle Doc-Kommentare sind Deutsch; die Testnamen sind Deutsch
    (`ingestion_id.rs:46`, `:53`; `non_empty_string.rs:33`, `:39`); Mechanik-Bezeichner sind
    Englisch, Domänen-Fachbegriffe deutsch-fachsprachlich. **Beobachtung (kein klarer
    Regelbruch):** die `Display`-Texte der Fehler sind Englisch
    (`error.rs:35–38`, `:76–82`, `:107–109`). Die Regel bindet ausdrücklich **Doc-Kommentare
    und Testnamen** an Deutsch und Bezeichner an Englisch; laufende `Display`-Meldungstexte
    fallen unter keine der beiden Kategorien, die Regel ist hier also nicht verletzt. Es ist
    dennoch eine Sprach-Inkonsistenz (Projektsprache Deutsch vs. englische Diagnosetexte),
    die eine bewusste Entscheidung verdient. → Folge-Ticket C.

### `.rules/rust/gates.md`

20. **Sechs Gates grün, keine toten Doc-Links — konform (verifiziert).** Alle sechs Gates
    (`build`, `test`, `clippy --all-targets`, `fmt --check`, `cargo test -p architecture`,
    `cargo doc --no-deps --workspace`) sind auf diesem Branch grün (siehe PR-Beschreibung /
    `/cargo-six-gates`-Lauf). Die zahlreichen Intra-Doc-Links des Crates verweisen auf
    öffentliche Items; `cargo doc` bricht nicht. **Hinweis zu Befund 7/17:** Das
    „Don't Panic"-Clippy-Set (Workspace-`Cargo.toml`) aktiviert u. a. `panic`,
    `unwrap_used`, `panic_in_result_fn` — **nicht** aber `clippy::assert`. Der `assert!`-Panic
    in `IngestionId::new` (`value_objects/ingestion_id.rs:26`) wird darum **nicht** vom
    Lint-Netz erfasst; die Gates bleiben trotz des Panic-Pfads grün. Das ist der Grund, warum
    Befund 7 eine bewusste Bewertung (Folge-Ticket A) verdient, statt sich auf die Gates zu
    verlassen.

## Folge-Ticket-Vorschläge

Reine Vorschläge aus diesem Audit (nichts davon wird in diesem PR umgesetzt). Priorität
durchweg niedrig — es besteht **kein** harter Regelbruch.

### Folge-Ticket A — `IngestionId`-Panic-Pfad bewusst bewerten
- **Betroffene Regel:** `.rules/rust/idiome.md` („Newtypes validieren beim Konstruieren")
  i. V. m. dem „Don't Panic"-Clippy-Set (`.rules/rust/gates.md`).
- **Ort:** `crates/einlesen_domain/src/value_objects/ingestion_id.rs:24–32`.
- **Inhalt:** Entscheiden und dokumentieren, ob der `assert!`-Panic bei leerer io-vergebener
  ID die gewollte Invarianten-Durchsetzung bleibt, oder ob ein `#[expect(clippy::…, reason=…)]`
  bzw. eine explizite Lint-Aktivierung (`clippy::assert`) den bewussten Charakter des
  Panic-Pfads für das Lint-Netz sichtbar machen soll. Kein funktionaler Zwang; dient der
  Konsistenz mit der „Don't Panic"-Haltung.

### Folge-Ticket B — `proptest` für die einfachen Domänen-Invarianten
- **Betroffene Regel:** `.rules/rust/testing-werkzeuge.md` („Property-based Tests mit proptest").
- **Ort:** `crates/einlesen_domain/src/value_objects/non_empty_string.rs`,
  `…/amount.rs` (bzw. die Aggregat-Specs in `crates/einlesen_specs/`).
- **Inhalt:** Property-Tests über den Eingaberaum ergänzen (z. B. `NonEmptyString`-Trim ist
  idempotent und nie leeres Ergebnis; `Amount` bleibt über den `DecodedAmount`-Eingaberaum
  vorzeichenlos). „Soweit sinnvoll" — bewusst optional, kleiner Umfang.

### Folge-Ticket C — Sprache der Fehler-`Display`-Texte festlegen
- **Betroffene Regel:** `.rules/common/sprache.md` (Randfall: laufende Meldungstexte, von der
  Regel nicht ausdrücklich erfasst).
- **Ort:** `crates/einlesen_domain/src/error.rs` (alle `Display`-Impls) — und, falls
  entschieden, projektweit konsistent.
- **Inhalt:** Eine bewusste Entscheidung treffen und ggf. in `.rules/common/sprache.md`
  festhalten, ob laufende `Display`-Diagnosetexte Deutsch oder Englisch sein sollen, damit die
  Konvention nicht am Einzelfall driftet. Rein redaktionell/konventionell.
