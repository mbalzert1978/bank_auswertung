---
id: "0011"
title: "Kategorisierer Chat-Payload-Adapter"
status: open
---

# Kategorisierer Chat-Payload-Adapter

**Typ:** AFK

> GitHub-Issue: [#98](https://github.com/mbalzert1978/bank_auswertung/issues/98)

## Was zu bauen ist

Der `Kategorisierer`-Port (outbound), Chat-Payload-Adapter zuerst
(Baureihenfolge-Schritt 5): exportiert **nur die der Domäne noch unbekannten**
Fingerabdrücke (Abgleich gegen das Kategorie-Register aus Slice 0009) als JSON
(`KategorieAnfrage`: Fingerabdruck + `ObfuskierterText`), der Nutzer gibt das
in einen Chat, die Antwort (`KategorieAntwort`: Fingerabdruck → Kategorie)
wird zurück importiert und ins Kategorie-Register übernommen.

`KategorieAntwort` schlüsselt ausschließlich auf dem Fingerabdruck, nie auf
dem Text — Re-Identifikation ist damit strukturell ausgeschlossen. Bei
Zahlungsintermediären muss die exportierte `ObfuskierterText`-Nutzlast aus dem
Verwendungszweck stammen, nicht aus dem (uninformativen) Empfänger-Namen des
Intermediärs — das nutzt die Intermediär-Erkennung aus Slice 0005.

> **Vorbedingung (Entscheidung des Orchestrators, 2026-10-12):** das Kategorie-Register aus
> [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md) ist die **einzige Wahrheit** für
> Kategorie-Etiketten. Der Import schreibt **ausschließlich** über
> `io::auswerten::SqliteCategoryRegistry::learn(&[CategoryAnswerRow])` — alles oder nichts, und nur
> Fingerabdrücke, die eine Auswertung schon geführt hat (`window_evaluation_fingerprints`); ein
> erfundener Schlüssel, etwa ein Klarname aus der Chat-Antwort, wird redigiert abgewiesen
> (`RejectCause::UnknownFingerprint`). Welche Fingerabdrücke unbekannt sind, beantwortet derselbe
> Register-Seam (`known_among`).

## Abnahmekriterien

- [x] Export enthält ausschließlich Fingerabdrücke, die im Kategorie-Register
      noch **keine** Kategorie haben — bereits bekannte Fingerabdrücke werden
      nicht erneut exportiert.
      *(Beleg: `run::kategorien_tests::der_round_trip_lernt_…_und_der_naechste_export_fragt_nur_den_rest`,
      `…::ein_liegengebliebener_alter_lauf_steht_nicht_im_export`)*
- [x] Export-Payload enthält für Buchungen mit erkanntem
      Zahlungsintermediär den Verwendungszweck (nicht den Empfänger-Namen)
      als `ObfuskierterText`.
      *(Beleg: `auswerten_specs/tests/kategorie_anfrage.rs::beim_durchleiter_geht_der_zweck_hinaus_nicht_der_name`,
      `run::kategorien_tests::der_export_traegt_nur_unbekannte_firmen_und_durchleiter_und_keinen_rohwert`)*
- [x] Import einer `KategorieAntwort`-Datei ordnet Kategorien korrekt über
      den Fingerabdruck zu und schreibt sie ins Kategorie-Register (Slice
      0009); eine erneute Analyse zeigt die gelernte Kategorie.
      *(Beleg: Round-Trip-Test oben — `categories_of` liefert das Etikett, der nächste Lauf fragt nicht
      mehr; `…::ein_bekanntes_etikett_bleibt_ohne_korrektur_und_eine_korrektur_ueberdauert_den_lauf`)*
- [x] Test: kein Feld in Export oder Import erlaubt eine Zuordnung über den
      Freitext statt über den Fingerabdruck — eine Antwort ohne passenden
      Fingerabdruck wird verworfen, nicht texto-heuristisch zugeordnet.
      *(Beleg: `auswerten::chat_import::tests::gelernt_wird_nur_ueber_den_exakten_fingerabdruck_nie_ueber_text`,
      Round-Trip-Test mit `fixtures/kategorien/antwort.json`)*
- [x] Test deckt den vollständigen Round-Trip ab: unbekannte Fingerabdrücke →
      Export-JSON → (kanonische Fixture-Antwort) → Import → aktualisiertes
      Kategorie-Register.
      *(Beleg: `run::kategorien_tests::der_round_trip_lernt_…`)*

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md) (Zahlungsintermediär-Erkennung
  für die Textwahl im Export)
- Blockiert durch [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
- Blockiert durch [0010](0010-scrubbing-regeln-fuer-obfuskiertertext.md)

## Entwurf (Datenschutz-Review vor dem Bau)

> Stand 2026-10-12. Erster Datenabfluss nach außen — darum zuerst der Entwurf, dann das Review, dann
> der Bau. Das Review (BLOCK, drei Blocker) und die Entscheidungen des Orchestrators sind
> **eingearbeitet**; § 9 hält fest, was entschieden wurde.

### 1. Aufruf

**Export = Option des Laufs**, nicht eigener Befehl:

```bash
bank-auswertung --db lauf.sqlite --csv giro.csv --fenster fenster.toml --seed seed.toml \
  --eigene-konten eigene-konten.toml --personen personen.toml \
  --kategorien-export kategorien-anfrage.json
```

- `--kategorien-export` ist **optional**; ohne sie läuft alles wie heute.
- Die Datei entsteht **nur bei Exit 0**, am Ende des Laufs, und enthält genau die Anfragen **dieses**
  Laufs — dieselbe Regel, die ADR-0016 (C) für den Bericht festlegt: kein nachträgliches Suchen per
  `max(run_id)`, kein Export eines halben Laufs. Bei Exit 1 entsteht keine Datei.
- Geschrieben wird in eine **neue** Nachbar-Datei mit Rechten `0600`, dann umbenannt. Scheitert das
  Schreiben, wird die Nachbar-Datei gelöscht und eine **alte** Export-Datei bleibt unberührt.
- Warum kein eigener Befehl `export`: den Text der Anfrage bildet die **Domäne** beim Auswerten. Ein
  späterer Befehl müsste ihn aus der Datenbank neu bilden — das kann io nicht (Fingerabdruck und Text
  sind Domänenwissen), oder es bräuchte eine neue Tabelle samt Schema-Migration. Ein Lauf mit
  derselben CSV ist seit ADR-0016 harmlos; wer die Option vergessen hat, wiederholt den Lauf.

**Import = eigener Befehl**, er braucht nur die Datenbank:

```bash
bank-auswertung kategorien-import --db lauf.sqlite --antwort kategorien-antwort.json
bank-auswertung kategorien-import --db lauf.sqlite --antwort korrektur.json --korrektur
```

- Ohne `--korrektur` lernt der Import nur Fingerabdrücke, die **noch unbekannt** sind; ein bekanntes
  Etikett bleibt (W2). Mit `--korrektur` ersetzt er auch bekannte Etiketten — der bewusste Weg für
  eine Korrektur.
- Exit `0` = gelesen; `1` = abgewiesen (Datei unlesbar oder zu groß, Format kaputt, Widerspruch,
  Register nicht beschreibbar, falsche Schema-Version) **oder** nichts gelernt, aber etwas verworfen;
  `2` = falsch aufgerufen.
- Protokoll auf stderr, nur Zählwerte und Positionen: `Kategorien-Import: 12 gelernt, 3 schon
  bekannt, 2 verworfen (Eintrag 4: Fingerabdruck nie ausgegeben; Eintrag 9: Kategorie ungültig), 1
  Zusatzfeld übergangen`. Nie ein Fingerabdruck, nie ein Etikett, nie ein Feldname aus der Datei.
- Danach fragt der nächste Lauf nach den gelernten Arten nicht mehr, und der Bericht (0012) liest ihr
  Etikett über `SqliteCategoryRegistry::categories_of`.

### 2. Format

**Export** (`kategorien-anfrage.json`), UTF-8, Einträge nach Fingerabdruck sortiert:

```json
{
  "format": "bank-auswertung/kategorie-anfrage/1",
  "anleitung": "Ordne jeder Anfrage genau eine kurze Kategorie zu (z. B. Lebensmittel, Wohnen, Mobilität). Antworte nur mit JSON in der Form {\"antworten\": [{\"fingerabdruck\": \"…\", \"kategorie\": \"…\"}]}. Übernimm jeden Fingerabdruck Zeichen für Zeichen. Lass eine Anfrage weg, wenn du sie nicht einordnen kannst.",
  "anfragen": [
    {
      "fingerabdruck": "COMPANY:DE98ZZZ09999999999",
      "text": "Stadtwerke Beispielstadt"
    },
    {
      "fingerabdruck": "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:MUSIKDIENST-BEISPIEL",
      "text": "Musikdienst Beispiel"
    }
  ]
}
```

Erfundene Quellzeilen dazu: Firma `Stadtwerke Beispielstadt` mit Zweck `Abschlag Strom Kd-Nr 4711`;
PayPal-Lastschrift mit Zweck `PP.4711.PP Musikdienst Beispiel`.

| Feld | Inhalt |
| --- | --- |
| `format` | feste Zeichenkette, Versionsziffer für spätere Änderungen |
| `anleitung` | fester Text im Programm, enthält keine Daten |
| `anfragen[]` | je unbekannter Buchungs**art** genau ein Eintrag mit genau zwei Feldern |
| `anfragen[].fingerabdruck` | `CategoryRequestRow::fingerprint()` |
| `anfragen[].text` | `CategoryRequestRow::text()` |

Nicht im Export: Lauf-ID, Datum, Betrag, Richtung, Konto-Token (`KONTO_…`), Personen-Token
(`PERSON_…`), Seed-Marker, Zahl der Buchungen.

**Import** (`kategorien-antwort.json`) — die Form, die die Anleitung verlangt:

```json
{
  "antworten": [
    { "fingerabdruck": "COMPANY:DE98ZZZ09999999999", "kategorie": "Wohnen & Energie" },
    { "fingerabdruck": "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:MUSIKDIENST-BEISPIEL", "kategorie": "Unterhaltung" }
  ]
}
```

| Feld | Inhalt |
| --- | --- |
| `antworten[]` | Liste; die Position (ab 1) ist das Einzige, was eine Meldung über einen Eintrag sagt |
| `antworten[].fingerabdruck` | Zuordnungsachse — **exakter** Vergleich |
| `antworten[].kategorie` | das Etikett: einzeilig, ohne Steuerzeichen, getrimmt 1–60 Zeichen (W3) |
| jedes weitere Feld | wird **nie gelesen**, nur gezählt |

### 3. Herkunft jedes exportierten Werts

| Wert | Herkunft | Schranke | Warum keine IBAN/BIC/kein Klarname/kein Seed |
| --- | --- | --- | --- |
| `COMPANY:<id>` | Spalte Gläubiger-ID | `CreditorId` wird in `obfuskieren` **geparst** (B2): Form der SEPA-Gläubiger-ID + Mod-97; eine ungültige gilt als **fehlend** → Person | eine IBAN, ein Name oder anderer Unsinn in der Spalte ist keine gültige Gläubiger-ID und erreicht den Fingerabdruck nicht; die gültige benennt den **Einzieher**, nicht den Kontoinhaber (bewusste Ausnahme, CONTEXT.md) |
| `MERCHANT:<token>` | Verwendungszweck nach `ObfuskierterText`, dann die **Egress-Wortregel** (§ 4), dann Wörter großgeschrieben | `ObfuskierterText` + Egress-Wortregel | eine IBAN ist im Text schon ersetzt; eine nicht erkannte trüge Ziffern, eine E-Mail ein `@` — beide fallen **als ganzes Wort**, bevor zerlegt wird (B1). Ein **unbekannter** Name bleibt stehen (§ 9) |
| `text` | gescrubbter Firmenname bzw. gescrubbter Zweck eines Durchleiters (§ 4), danach dieselbe Egress-Wortregel | `ObfuskierterText` + Egress-Wortregel | wie oben; die Regel nimmt dazu Kunden-, Vertrags-, Rechnungs-, Telefonnummern und Datumsangaben mit |
| `format`, `anleitung` | Konstanten im Code | — | enthalten keine Daten |

Personen gehen **nicht** hinaus (§ 9, Entscheidung a). Damit verlässt **kein** vom Seed abgeleiteter
Wert die Maschine — weder ein `PERSON_…`- noch ein `KONTO_…`-Token, noch der Seed-Marker.

### 4. Gegenpartei einmal einordnen, Text und Fingerabdruck daraus

Die Domäne ordnet die Gegenpartei **einmal** ein, als Summentyp (W4):

| Art | erkannt an | Fingerabdruck | Anfrage-Text |
| --- | --- | --- | --- |
| **Durchleiter** | Firma, deren Gläubiger-ID (zuerst) oder Name einen Durchleiter nennt | `COMPANY:<id>` + `\|MERCHANT:<token>`, wenn der Zweck einen Token ergibt | **nur** der Zweck (AK 2); ohne verwertbaren Zweck **keine** Anfrage |
| **Firma** | Gläubiger-ID vorhanden | `COMPANY:<id>` | **nur** der Name; ohne Namen **keine** Anfrage (kein Rückfall auf den Zweck) |
| **Person** | Name ohne Gläubiger-ID | `PERSON:<pseudonym>` | **keine** Anfrage |
| **Keine** | weder noch (Entgelt) | keiner | keine Anfrage |

Fingerabdruck und Anfrage-Text sind zwei vollständige `match`-Funktionen auf diesem Summentyp.

**Die Egress-Wortregel gibt es genau einmal** (B1): zerlegt an Leerraum, fällt jedes Wort, das eine
Ziffer oder ein `@` trägt — **bevor** der Händler-Token weiter an Satzzeichen zerlegt. So wird aus
`PP.4711.PP max.mustermann@web.de Musikdienst` der Token `MUSIKDIENST`, nicht
`PP-PP-MAX-MUSTERMANN-WEB-DE-MUSIKDIENST`. Sie gilt für den Händler-Token **und** den Anfrage-Text.
Bleibt nichts übrig, gibt es keinen Token bzw. keine Anfrage. Das ändert bestehende Fingerabdrücke
von Durchleitern; das Register ist noch leer, die Änderung kostet also nichts.

Pro Art gewinnt wie heute der **erste** Text des Fensters; über die Fenster des Laufs hinweg
ebenfalls der erste (Reihenfolge der Outbox, für dieselbe Datei gleich).

### 5. Was „unbekannt" heißt und welcher Lauf

1. Je Fenster bildet die Domäne die Anfragen (eine je Fingerabdruck) und fragt das Register
   (`known_among`, Issue 0009). Nur der Rest geht an den `Categorizer`-Port.
2. In io erfüllt den Port statt `UnansweredCategorizer` ein **sammelnder Kategorisierer** — ein
   Zustand mit zwei Fällen: *ruhend* (antwortet leer, merkt sich nichts) und *sammelnd* (merkt sich
   die Anfragen, antwortet leer). Der Lauf rechnet weiter wie heute (ADR-0004).
3. Er sammelt **erst ab Phase 2** des Laufs. Phase 1 kann einen liegengebliebenen alten Lauf zu Ende
   führen; dessen Anfragen gehören nicht in diesen Export.
4. Am Ende (Exit 0) fragt io das Register ein zweites Mal (`known_among`) und lässt Bekanntes weg.

### 6. Wie der Compiler den Leak verhindert

- `CategoryRequestRow` bekommt **private** Felder mit Lese-Methoden; ihr **einziger** Konstruktor
  ist `From<&CategoryRequest>`, und eine `CategoryRequest` kann nur die Domäne bauen
  (`pub(crate) new`). io kann also keine Anfrage-Zeile aus eigenen Zeichenketten erfinden (B3).
- Der Guard `assert_not_impl_any!(CategoryRequestRow: Serialize, Deserialize<'static>)` bleibt.
- Der **Export-Schreiber** ist ein eigenes schmales io-Modul. Seine Signatur nimmt nur das Ziel und
  `&[CategoryRequestRow]` — keine Config, keine Datenbank, keinen Seed. Das JSON baut er von Hand als
  `serde_json::Value` aus genau den zwei benannten Feldern. Kein Typ im Export- und Import-Pfad leitet
  `Serialize`/`Deserialize` ab; neu ist nur `serde_json` in `crates/io`.

### 7. Import: Zuordnung nur über den Fingerabdruck

1. Datei größer als 1 MiB, kein gültiges JSON, oder oben kein Objekt mit `antworten`-Liste →
   **ganze Datei abgewiesen**. Meldung: statische Ursache plus Zeile/Spalte, nie der Fehlertext von
   `serde_json`.
2. Ein Eintrag ist kein Objekt, oder `fingerabdruck`/`kategorie` fehlt oder ist kein Text → Eintrag
   **verworfen**. Weitere Felder (`text`, `begruendung` …) werden **nie gelesen**, nur gezählt.
3. `kategorie` muss ein gültiges **`Kategorie`-Value-Object** sein (W3): getrimmt 1–60 Zeichen,
   einzeilig, keine Steuerzeichen — sonst Eintrag verworfen.
4. **Widerspruch** (W6): über alle formal gültigen Einträge — **vor** den Filtern unten — prüft der
   Import, ob ein Fingerabdruck zwei verschiedene Kategorien bekommt. Dann ist die **ganze Datei**
   abgewiesen, mit beiden Positionen aus der Datei.
5. `fingerabdruck` (getrimmt) ist nicht **exakt** gleich einem Fingerabdruck, den eine Auswertung
   ausgegeben hat (`window_evaluation_fingerprints`, neue Lese-Methode `issued_among`) → verworfen.
   Kein Vergleich ohne Groß/klein, kein Präfix, keine Ähnlichkeit, keine Suche über Text.
6. Ohne `--korrektur`: ein Fingerabdruck, den das Register schon kennt, wird **übergangen** (gezählt,
   nicht überschrieben).
7. Der Rest geht in **einem** Aufruf an `SqliteCategoryRegistry::learn` — alles oder nichts;
   `learn`s eigene Prüfungen bleiben das letzte Netz.

### 8. Tests (alle Daten erfunden)

In `crates/io` gegen echtes SQLite, über `io::run::run` mit eigener Fixture-CSV:

- **Leak-Test**, geprüft auf den rohen **und** normalisierten Bytes der Export-Datei (groß, ohne
  Trenner — so fällt auch `MAX-MUSTERMANN` auf): erfundene IBANs (Gegenkonto, im Zweck kompakt und
  gruppiert), eine BIC (Spalte und im Zweck), ein Name aus `--personen` und eine E-Mail im Zweck eines
  **Durchleiters**, ein Empfänger ohne Gläubiger-ID, Spalte 5 mit einer IBAN bzw. einem Namen, eine
  Kundennummer. Nicht im Export: keiner dieser Werte, kein `KONTO_…`/`PERSON_…`-Token und nicht der
  Seed-Marker aus `obfuskiert` — jeweils auch als bloße 32 Hex-Zeichen —, nicht der Seed. Jeder
  Eintrag hat genau `fingerabdruck` und `text`.
- **Ehrliche Gegenprobe:** eine ausländische BIC ohne Hinweis bleibt stehen (bekannte Lücke aus 0010).
- **Textwahl:** Durchleiter → Zweck, nicht „PayPal …"; Durchleiter ohne Zweck → keine Anfrage; Firma
  ohne Namen → keine Anfrage; Person → keine Anfrage.
- **Nur dieser Lauf, nur Unbekanntes:** ein liegengebliebener alter Lauf steht nicht im Export; ein
  bekannter Fingerabdruck auch nicht.
- **Round-Trip:** Lauf mit Export → kanonische Fixture-Antwort → Import → `categories_of` → zweiter
  Lauf: die gelernten Arten stehen nicht mehr im Export.
- **Kein Überschreiben / Korrektur:** ohne `--korrektur` bleibt ein bekanntes Etikett; mit
  `--korrektur` wird es ersetzt und bleibt nach einem weiteren Lauf.
- **Keine Text-Zuordnung:** falscher Fingerabdruck + passender `text` → verworfen; Fingerabdruck in
  anderer Groß-/Kleinschreibung → verworfen; richtiger Fingerabdruck + `begruendung` → gelernt.
- **Widerspruch, kaputtes JSON, zu große Datei, ungültige Kategorie** → wie in § 7.
- **Protokoll:** stderr des Imports trägt weder Fingerabdruck noch Etikett.
- **Exit 1** → keine Export-Datei; alte Datei bleibt.

In `obfuskierung_specs`: Spalte 5 mit IBAN bzw. Namen → Person. In `auswerten_specs`: die Einordnung
aus § 4 und die Egress-Wortregel. In `crates/lauf`: Option und Unterbefehl.

### 9. Entscheidungen und bewusste Grenzen

Entschieden (Orchestrator, 2026-10-12):

- **(a) Personen werden nicht exportiert.** Preis: Kartenzahlungs-Händler tragen keine Gläubiger-ID
  und gelten darum fail-safe als Person — **alle Kartenzahlungen** fehlen im Chat, ebenso
  Überweisungen an Personen (Miete an einen privaten Vermieter, Taschengeld).
- **(b) Firma nur mit Namen**, ohne Namen keine Anfrage.
- **(c) Zusatzfelder** einer Antwort werden nie gelesen, nur gezählt.

**Bewusste Grenzen** (ehrlich benannt, keine Anonymisierung):

- Ein **starker, zufälliger Seed** ist Voraussetzung für jeden späteren Export seed-abgeleiteter Werte
  (heute ist der Seed ein frei gewählter `u64`, `0` ist gültig). Gehört in die Checkliste von
  Schritt 6.
- Eine **natürliche Person mit gültiger Gläubiger-ID** (privater Vermieter, Verein,
  Einzelunternehmer) gilt als Firma; ihr Name geht als `text` hinaus. Abhilfe: den Namen vor dem
  ersten Lauf in `personen.toml` eintragen — dann wird er auch im Firmennamen zu `[PERSON]`.
- Ein **unbekannter Name** — weder Empfänger ohne Gläubiger-ID noch in `personen.toml` — bleibt im
  Text **und im Händler-Token** stehen (etwa ein PayPal-Freund im Zweck). Abhilfe: PayPal-Freunde,
  Vornamen und den **eigenen** Namen in `personen.toml` eintragen; das sagt auch der Hilfetext.
- Der Chat-Anbieter sieht das **Gläubiger-Profil** des Haushalts: welche Firmen einziehen (Versicherer,
  Vermieter, Vereine, Streaming). Das ist der Preis der Kategorisierung per Chat.
- Eine ausländische BIC ohne Ziffer und ohne `BIC`-Hinweis bleibt im Text stehen (Lücke aus 0010).
- Die Export-Datei ist lesbar. Der Nutzer trägt sie selbst in den Chat — **vorher ansehen** ist die
  letzte Schranke, und der Hilfetext sagt das.

## Wie es gebaut wurde

Fünf Commits auf `0011-chat-export`: zwei für den Entwurf, drei für den Bau.

- **Gläubiger-ID geparst** (`obfuskierung_domain`, `value_objects/creditor_id.rs`): Form der
  SEPA-Gläubiger-ID plus Mod-97 ohne Geschäftsbereich, ohne Fremd-Crate. Eine ungültige gilt als
  fehlend, die Gegenpartei wird fail-safe Person. Dieselbe Regel (`is_valid_creditor_id`, über den
  Adapter re-exportiert) entscheidet in io, welche Empfänger-Namen auf die Namensliste des Scrubbings
  kommen. Die erfundenen Gläubiger-IDs der io-Fixtures und `obfuskierung_specs` tragen jetzt gültige
  Prüfziffern; die echten Durchleiter-IDs sind gültig und werden weiter erkannt.
- **Gegenpartei einmal eingeordnet** (`auswerten_domain`, `value_objects/party.rs`): `Party` =
  Durchleiter / Firma / Person / keine. `Fingerprint::of` und `Counterparty::category_text` sind zwei
  vollständige `match`es darauf. Die **Egress-Wortregel** steht genau einmal (`egress_words`) und
  läuft vor dem Zerlegen — im Händler-Token und im Anfrage-Text. Folge: Durchleiter-Fingerabdrücke
  verlieren das `PP-PP-`-Präfix (`MERCHANT:REWE-MARKT` statt `MERCHANT:PP-PP-REWE-MARKT`); das Register
  war leer, also kostet das nichts.
- **`CategoryRequestRow`** hat private Felder und nur `From<&CategoryRequest>` als Konstruktor; die
  Spec-Fixture vergleicht jetzt Paare (Fingerabdruck, Text).
- **io:** `CollectingCategorizer` (Zustand ruhend/sammelnd) ersetzt `UnansweredCategorizer`;
  `auswerten::chat_export` (nur Ziel + Zeilen, JSON von Hand, `0600`, Nachbar-Datei + Umbenennen);
  `auswerten::chat_import` (1 MiB, `Kategorie`-Value-Object, Widerspruch vor den Filtern,
  `issued_among` neu am Register, `known_among` ohne `--korrektur`, dann `learn`); `run::kategorien`
  verdrahtet beides; `serde_json` neu in `crates/io`, kein `serde`-Derive.
- **lauf:** optionale `--kategorien-export`, Unterbefehl `kategorien-import --db --antwort
  [--korrektur]`; Hilfetext nennt, was hinausgeht, was fehlt, und wen man in `personen.toml` einträgt.
- Ticket 0012 hat ein AK „Etikett im HTML escapen" bekommen; der Abschlussplan (Schritt 6) nennt den
  starken Seed und die zusätzlichen Namen für `personen.toml`.

Keine Schema-Änderung, keine Migration: gesammelt wird im Speicher, nicht in einer Tabelle.
