---
id: "0009"
title: "Speicher (SQLite) für DTOs und Kategorie-Register"
status: open
---

# Speicher (SQLite) für DTOs und Kategorie-Register

**Typ:** AFK

> GitHub-Issue: [#96](https://github.com/mbalzert1978/bank_auswertung/issues/96)

> **Hinweis zum Zuordnungs-Speicher:** Seit
> [ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md) gibt es **keinen**
> persistenten Klartext↔Pseudonym-Speicher mehr (Slice 0008 hinfällig). Das lokal zu
> haltende Geheimnis ist der **config-injizierte Seed**. Die Kern-Invariante unten bleibt:
> der `Speicher`-Port hält **ausschließlich obfuskierte DTOs** und **nie** den Seed/Rohwert;
> die AK-Punkte zur „Trennung vom Zuordnungs-Speicher (Slice 0008)" lesen sich heute als
> „der `Speicher` trägt den Seed nicht". Die Vorbedingung „setzt Slice 0008 voraus" entfällt.

## Was zu bauen ist

Der `Speicher`-Port (outbound), SQLite-Adapter (Baureihenfolge-Schritt 4):
persistiert **DTOs** (nicht Domänenobjekte) über mehrere Importe hinweg —
strikt getrennt vom Zuordnungs-Speicher aus Slice 0008 (nie mischen).

Enthält das **Kategorie-Register** (D5): schlüsselt auf dem `Fingerabdruck`,
speichert gelernte Korrekturen. Das Mapping DTO ↔ Domäne lebt im
Anti-Corruption-Layer, der das Kategorie-Register beim Laden aus den
gespeicherten DTOs rekonstituiert.

Setzt die persistente Pseudonym-Zuordnung (Slice 0008) voraus, damit
`PERSON_x`-Token schon beim ersten dauerhaft gespeicherten Import
laufstabil sind — sonst würde ein späterer Wechsel von laufstabilen zu
importübergreifenden Pseudonymen bereits gespeicherte Historie inkonsistent
machen.

## Abnahmekriterien

- [x] `Buchung`-DTOs aus zwei separaten CSV-Importen werden in SQLite
      persistiert und beim Neustart des Programms vollständig wieder geladen.
      — `run::speicher_tests::zwei_importe_werden_persistiert_und_beim_neustart_vollstaendig_geladen`
- [x] Das Kategorie-Register schlüsselt auf `Fingerabdruck`; eine gelernte
      Korrektur (Fingerabdruck → Kategorie) übersteht einen Neustart und wird
      beim nächsten Laden automatisch wieder angewendet.
      — `run::speicher_tests::eine_gelernte_korrektur_uebersteht_den_neustart_und_das_orakel_fragt_nicht_mehr_danach`,
      `auswerten::registry::tests::eine_gelernte_kategorie_uebersteht_einen_neustart`,
      `…::eine_korrektur_ersetzt_das_alte_etikett_und_uebersteht_den_neustart`
- [x] Der Anti-Corruption-Layer rekonstituiert beim Laden Domänenobjekte aus
      den gespeicherten DTOs, ohne dass die Domäne SQLite oder das
      Speicherformat kennt.
      — die `RegistryBridge` (`auswerten_adapter`) macht aus den Schlüssel-Zeichenketten des Seams
      wieder Domänen-Fingerabdrücke, und zwar durch Abgleich mit den gestellten Kandidaten, nie durch
      Neubildung; `auswerten_domain` bindet kein Fremd-Crate (`cargo test -p architecture`). Verhalten:
      Spec `eine_bereits_bekannte_buchungsart_erreicht_das_orakel_nicht` (Fake) und der Korrektur-Test
      oben (reales Register, realer Consumer).
- [x] Test: der `Speicher`-Port speichert ausschließlich obfuskierte DTOs —
      kein Test-Fixture mit Klartext-Namen landet unverändert in der
      SQLite-Datei.
      — `run::speicher_tests::kein_klarname_steht_ausserhalb_der_rohtabelle_auch_nicht_im_register`
      (Lesart siehe unten)
- [x] Test belegt die strukturelle Trennung von Zuordnungs-Speicher (Slice
      0008) und diesem `Speicher`: unterschiedliche Speicherorte/Formate,
      kein gemeinsamer Zugriffspfad. — **gelesen als „der `Speicher` trägt den Seed nicht"**
      (siehe Hinweis oben): `run::speicher_tests::weder_register_noch_eine_andere_tabelle_traegt_den_seed`

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- ~~Blockiert durch [0008](0008-persistente-pseudonym-zuordnung.md)~~ — **entfällt**: 0008 ist
  durch ADR-0010 + 0059 erledigt und geschlossen.

## Wie es gebaut wurde

- **Das Register** ist `io::auswerten::SqliteCategoryRegistry`
  ([`crates/io/src/auswerten/registry.rs`](../../crates/io/src/auswerten/registry.rs)): eine Tabelle
  `learned_categories` mit dem **Fingerabdruck als Primärschlüssel** und dem Etikett — kein Text, aus dem
  das Etikett geraten wurde, also auch auf der Platte kein Rückweg vom Etikett zum Freitext. Es erfüllt
  den bestehenden `CategoryRegistrySeam` und ersetzt im Lauf (`io::run`) das frühere
  `EmptyCategoryRegistry`. Der Kategorisierer bleibt `UnansweredCategorizer` (der Chat-Weg ist
  [0011](0011-kategorisierer-chat-payload-adapter.md)); die Domäne fragt ihn schon heute nur für die
  Fingerabdrücke, die das Register nicht kennt (`WindowEvaluationHandler::categorise`).
- **Das Register ist die einzige Wahrheit für Kategorie-Etiketten** (Entscheidung des Orchestrators,
  2026-10-12, als Vorbedingung in [0011](0011-kategorisierer-chat-payload-adapter.md) und
  [0012](0012-html-bericht.md) eingetragen): der Chat-Import schreibt nur über `learn`, der Bericht liest
  nur über `categories_of`. Die Etiketten-Kopie im Fenster-Ergebnis (`window_evaluation_categories`)
  bleibt, ist aber nur der Mitschnitt des Orakels in jenem Lauf und **keine** Quelle für den Bericht.
- **Schreibweg für 0011:** `SqliteCategoryRegistry::learn(&[CategoryAnswerRow]) -> Result<(), LearnError>`
  — dieselbe Zeilen-Form, die das Orakel antwortet. Alle oder keiner, in einer Transaktion; Werte
  getrimmt; ein leerer Fingerabdruck, ein leeres Etikett, zwei **verschiedene** Etiketten für denselben
  Fingerabdruck im Stapel oder ein Fingerabdruck, den **keine Auswertung geführt** hat
  (`window_evaluation_fingerprints`, geprüft in derselben Transaktion) weisen den ganzen Stapel ab
  (`LearnError::Rejected { position, cause }`, redigiert: nur Position und statischer Grund). Damit
  kann ein erfundener Schlüssel — etwa ein Klarname aus einer Chat-Antwort — nie in die Datei
  (`ein_nie_ausgegebener_schluessel_wird_abgewiesen_und_nicht_genannt`). Ein schon bekannter Fingerabdruck wird
  **korrigiert** (Upsert; `updated_at` rückt nur bei einem neuen Etikett nach). Lesen für den Bericht
  (0012): `categories_of(&[String]) -> Vec<CategoryAnswerRow>`.
- **Kategorie = freier, validierter Text, kein festes Vokabular.** So hält es die Domäne schon:
  `Category` ist ein `NonEmptyString`, weil die Schubladen eine Aussage des Orakels bzw. des Menschen
  sind, nicht der Analyse — ein `enum` machte den Slice bei jeder neuen Schublade änderungsbedürftig.
  Das Register prüft dasselbe (getrimmt, nicht leer), ein `CHECK` hält es auf DB-Ebene.
- **Das Ergebnis eines Fensters trägt weiter nur die Etiketten, die das Orakel in diesem Lauf
  beigetragen hat** — eine bewusste, schon bestehende Entscheidung der Specs: was das Register führt,
  gehört ihm, und an jedes Fenster geheftet wäre es eine Kopie. „Wird wieder angewendet" heißt also: die
  Art gilt als bekannt und geht nicht mehr ans Orakel; ihr Etikett liest der Bericht aus dem Register.
- **Schema-Version 3** ([`crates/io/src/schema.rs`](../../crates/io/src/schema.rs)) über eine kleine
  **Leiter**: jede Sprosse hebt um genau eine Version (feste Zielversion), `ensure` steigt von der
  vorgefundenen Version bis `SCHEMA_VERSION` und stempelt einmal am Ende; ein Test hält die Leiter
  lückenlos. Erste Sprosse 2 → 3 (`add_category_register`): Register-Tabelle plus Index auf
  `window_evaluation_fingerprints(fingerprint)` — rein additiv, die Version-2-Datei behält ihren
  Bestand und trägt danach genau das Schema einer frischen (Test
  `eine_version_2_datei_mit_bestand_bekommt_das_register_und_behaelt_ihren_bestand`). Version 1 ist
  keine Sprosse, sondern ein Neubau (`rebuild_unkeyed_v1`), der direkt auf der aktuellen Version
  landet. Eine weitere Migration (etwa die von 0085) hängt sich als nächste Sprosse an. Die Tabelle steht in `schema::tables()` unter
  den Nicht-Store-Tabellen (neben der Outbox); im Lauf lebt das Register unter den geliehenen
  `Adapters`, nicht unter `Stores` (es schreibt im Lauf nichts).
- **AK 1 brauchte keinen neuen Speicher:** die realen Stores (0053–0058, 0088) persistieren schon. Der
  Test fährt zwei Läufe mit zwei Auszügen in eine Datei (jeder Lauf öffnet und schließt sie = Neustart),
  öffnet sie danach erneut und lädt den Bestand über den realen Lese-Weg (`AuswertungsfensterSource`):
  alle 61 Buchungen, beide Auftragskonten.
- **Lesart AK „kein Klartext-Name in der SQLite-Datei":** die Rohtabelle `einlesen_lines` hält die Zeile
  lokal so, wie sie im Auszug steht — das ist erlaubt
  ([ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md),
  [ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md): die Datenschutz-Grenze ist die
  Maschinen-Grenze, und `obfuskieren` braucht die Rohzeile). Das AK gilt darum für **jede andere
  Tabelle** — alles stromabwärts von `obfuskieren` und das Register. Der Test liest jede Zelle jeder
  Tabelle (über SQLites eigene Tabellenliste) und prüft, dass die erfundenen Personen der
  Scrubbing-Fixture nur in `einlesen_lines` stehen (dort als Gegenprobe, dass die Suche greift).
- **Seed:** der Test fährt einen Lauf mit einem markanten Seed (`fixtures/lauf/seed_markant.toml`),
  lernt einen Register-Eintrag und prüft: keine Spalte heißt `seed`, keine Zelle irgendeiner Tabelle
  enthält ihn, und die Datei enthält ihn Byte für Byte nicht. Der Seed-**Marker** in `obfuskiert` ist ein
  Hash, kein Seed.

