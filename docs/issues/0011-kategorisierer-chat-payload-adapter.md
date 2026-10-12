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

- [ ] Export enthält ausschließlich Fingerabdrücke, die im Kategorie-Register
      noch **keine** Kategorie haben — bereits bekannte Fingerabdrücke werden
      nicht erneut exportiert.
- [ ] Export-Payload enthält für Buchungen mit erkanntem
      Zahlungsintermediär den Verwendungszweck (nicht den Empfänger-Namen)
      als `ObfuskierterText`.
- [ ] Import einer `KategorieAntwort`-Datei ordnet Kategorien korrekt über
      den Fingerabdruck zu und schreibt sie ins Kategorie-Register (Slice
      0009); eine erneute Analyse zeigt die gelernte Kategorie.
- [ ] Test: kein Feld in Export oder Import erlaubt eine Zuordnung über den
      Freitext statt über den Fingerabdruck — eine Antwort ohne passenden
      Fingerabdruck wird verworfen, nicht texto-heuristisch zugeordnet.
- [ ] Test deckt den vollständigen Round-Trip ab: unbekannte Fingerabdrücke →
      Export-JSON → (kanonische Fixture-Antwort) → Import → aktualisiertes
      Kategorie-Register.

## Blockiert durch

- Blockiert durch [0001](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)
- Blockiert durch [0005](0005-wiederkehr-erkennung.md) (Zahlungsintermediär-Erkennung
  für die Textwahl im Export)
- Blockiert durch [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md)
- Blockiert durch [0010](0010-scrubbing-regeln-fuer-obfuskiertertext.md)

## Entwurf (Datenschutz-Review vor dem Bau)

> Stand 2026-10-12, **noch nicht gebaut**. Erster Datenabfluss nach außen — darum zuerst dieser
> Entwurf, dann das Review, dann der Bau. Drei Stellen brauchen eine Entscheidung (§ 9).

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
  `max(run_id)`, kein Export eines halben Laufs. Bei Exit 1 entsteht keine Datei; eine alte bleibt
  unberührt.
- Geschrieben wird in eine Nachbar-Datei und dann umbenannt — eine halb geschriebene Anfrage gibt es
  nicht. Eine vorhandene Datei wird ersetzt.
- Warum kein eigener Befehl `export`: den Text der Anfrage bildet die **Domäne** beim Auswerten
  (`AnalysisWindow::category_requests`). Ein späterer Befehl müsste ihn aus der Datenbank neu bilden
  — das kann io nicht (Fingerabdruck und Text sind Domänenwissen), oder es bräuchte eine neue Tabelle
  samt Schema-Migration (Konflikt mit 0085). Ein Lauf mit derselben CSV ist seit ADR-0016 harmlos;
  wer die Option vergessen hat, wiederholt einfach den Lauf.

**Import = eigener Befehl**, er braucht nur die Datenbank:

```bash
bank-auswertung kategorien-import --db lauf.sqlite --antwort kategorien-antwort.json
```

- Exit `0` = gelesen (auch wenn Einträge verworfen wurden), `1` = abgewiesen (Datei unlesbar,
  Format kaputt, Widerspruch, Register nicht beschreibbar, falsche Schema-Version), `2` = falsch
  aufgerufen.
- Protokoll auf stderr, nur Zählwerte und Positionen: `Kategorien-Import: 12 gelernt, 2 verworfen
  (Eintrag 4: kein ausgegebener Fingerabdruck; Eintrag 9: leere Kategorie)`. Nie ein Fingerabdruck,
  nie ein Etikett, nie ein Feldname aus der Datei.
- Danach: der nächste Lauf fragt nach den gelernten Arten nicht mehr, und der Bericht (0012) liest
  ihr Etikett über `SqliteCategoryRegistry::categories_of`.

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
      "fingerabdruck": "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-MUSIKDIENST-BEISPIEL",
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
| `anfragen[].fingerabdruck` | `CategoryRequestRow::fingerprint` |
| `anfragen[].text` | `CategoryRequestRow::text` |

Nicht im Export: Lauf-ID, Datum, Betrag, Richtung, Konto-Token (`KONTO_…`), Seed-Marker, Zahl der
Buchungen.

**Import** (`kategorien-antwort.json`) — genau die Form, die die Anleitung verlangt:

```json
{
  "antworten": [
    { "fingerabdruck": "COMPANY:DE98ZZZ09999999999", "kategorie": "Wohnen & Energie" },
    { "fingerabdruck": "COMPANY:LU96ZZZ0000000000000000058|MERCHANT:PP-PP-MUSIKDIENST-BEISPIEL", "kategorie": "Unterhaltung" }
  ]
}
```

| Feld | Inhalt |
| --- | --- |
| `antworten[]` | Liste; die Position (ab 1) ist das Einzige, was eine Meldung über einen Eintrag sagt |
| `antworten[].fingerabdruck` | Zuordnungsachse — **exakter** Vergleich |
| `antworten[].kategorie` | das Etikett, getrimmt, nicht leer (wie `learn` es verlangt) |

### 3. Herkunft jedes exportierten Werts

| Wert | Herkunft | Schranke | Warum keine IBAN/BIC/kein Klarname/kein Seed |
| --- | --- | --- | --- |
| `COMPANY:<id>` | Spalte Gläubiger-ID, unverändert durch `obfuskieren` | bewusste Ausnahme: öffentliche Firmenkennung (CONTEXT.md, ADR-0010) | die Gläubiger-ID benennt den **Einzieher**, nicht den Kontoinhaber; IBAN/BIC stehen in eigenen Spalten, die `obfuskieren` tokenisiert bzw. verwirft |
| `MERCHANT:<token>` | Verwendungszweck nach `ObfuskierterText` (IBAN → `[IBAN]`, BIC → `[BIC]`, bekannte Namen → `[PERSON]`), dann `merchant_token`: großgeschrieben, **ohne jedes Wort mit Ziffer** | `ObfuskierterText` + Ziffern-Filter | eine IBAN ist im Text schon ersetzt; eine nicht erkannte trüge Ziffern und fällt am Filter. Ein **unbekannter** Name bleibt stehen (Grenze, § 8) |
| `PERSON:PERSON_<hex>` | SipHash-1-3 über (Seed, Name), 128 Bit | Pseudonym (ADR-0010) | kein Byte des Namens reist mit; der Seed bleibt lokal. Nur so stark wie der Seed — siehe § 9, Frage 1 |
| `text` | gescrubbter Firmenname bzw. gescrubbter Verwendungszweck (Wahl siehe § 4), danach in der **Domäne** jedes Wort mit Ziffer oder `@` entfernt | `ObfuskierterText` + Wort-Filter | wie oben; der Wort-Filter nimmt zusätzlich Kunden-, Vertrags-, Rechnungs-, Telefonnummern, Datumsangaben, Reste einer verfremdeten IBAN und E-Mail-Adressen mit — alles, was für die Kategorie nichts sagt, aber eine Person verraten kann |
| `format`, `anleitung` | Konstanten im Code | — | enthalten keine Daten |

Der Seed erscheint in keinem Feld; aus ihm abgeleitet ist allein das Pseudonym in `PERSON:…`.

### 4. Welcher Text — Durchleiter, Firma, Person

Heute (`Counterparty::category_text`) ist der Text **Name + Zweck** für jede Gegenpartei. Das ändert
sich in der Domäne (`auswerten_domain`, Regel neben dem Fingerabdruck, dieselbe `is_intermediary`):

| Gegenpartei | Text | Fehlt das Feld |
| --- | --- | --- |
| **Durchleiter** (PayPal, Amazon-Firmierungen, Klarna — erkannt wie beim Fingerabdruck, zuerst über die Gläubiger-ID) | **nur** der Verwendungszweck — der Name „PayPal Europe …" sagt nichts über die Kategorie (AK 2, CONTEXT.md „Zahlungsintermediär") | kein Zweck → **keine** Anfrage |
| **Firma** (mit Gläubiger-ID) | **nur** der Firmenname | kein Name → der Zweck |
| **Person** (ohne Gläubiger-ID) | siehe § 9, Frage 1 | — |
| keine Gegenpartei (Entgelt) | keine Anfrage (kein Fingerabdruck), wie heute | — |

Neu ist zweierlei, beides **Datenminimierung**: bei einer gewöhnlichen Firma fällt der Zweck weg
(dort stehen Kundennummer, Vertragsnummer, oft der Name des Kontoinhabers; die Firma ist das
Kategorie-Signal), und aus jedem Text fällt jedes Wort mit Ziffer oder `@`. Bleibt danach nichts
übrig, gibt es keine Anfrage. Pro Art gewinnt wie heute der **erste** Text des Fensters; über die
Fenster des Laufs hinweg ebenfalls der erste (Reihenfolge der Outbox, also für dieselbe Datei
gleich).

### 5. Was „unbekannt" heißt und welcher Lauf

1. Je Fenster bildet die Domäne die Anfragen (eine je Fingerabdruck) und fragt das Register
   (`known_among`, Issue 0009). Nur der Rest geht an den `Categorizer`-Port — so ist es schon gebaut.
2. In io erfüllt den Port statt `UnansweredCategorizer` ein **sammelnder Kategorisierer**: er merkt
   sich die Anfragen im Speicher und antwortet leer (der Lauf rechnet weiter wie heute, ADR-0004).
3. Er sammelt **erst ab Phase 2** des Laufs. Phase 1 kann einen liegengebliebenen alten Lauf zu
   Ende führen; dessen Anfragen gehören nicht in diesen Export. Phase 2 verarbeitet nur Nachrichten
   des eben geminteten Laufs.
4. Am Ende (Exit 0) fragt io das Register ein zweites Mal (`known_among`) und lässt Bekanntes weg —
   eine doppelte Sicherung, falls sich das Register während des Laufs geändert hätte.

Damit stehen im Export genau die Fingerabdrücke dieses Laufs, die das Register nicht kennt (AK 1).

### 6. Wie der Compiler den Leak verhindert

- Der einzige Weg in die Datei ist der Orakel-Seam: der sammelnde Kategorisierer bekommt
  ausschließlich `&[CategoryRequestRow]`, und der Export-Schreiber nimmt ausschließlich, was er
  gesammelt hat. Eine `CategoryRequestRow` entsteht im Adapter nur aus einer `CategoryRequest`, und
  die kann nur die Domäne bauen (`pub(crate) new`).
- `CategoryRequestRow` trägt schon den Guard `assert_not_impl_any!(CategoryRequestRow: Serialize,
  Deserialize<'static>)` (`crates/auswerten_adapter/src/lib.rs`). Er bleibt.
- Im ganzen Export- und Import-Pfad leitet **kein** Typ `Serialize` oder `Deserialize` ab. Das JSON
  baut io von Hand als `serde_json::Value` aus genau den zwei benannten Feldern
  (`"fingerabdruck": row.fingerprint`, `"text": row.text`). Bekommt die Zeile später ein drittes
  Feld, landet es **nicht** automatisch im Export.
- Neue Abhängigkeit: `serde_json` in `crates/io` (heute ist dort weder `serde` noch `serde_json`).
  Kein `serde`-Derive.
- Ein Test prüft zusätzlich, dass jeder Eintrag genau diese zwei Schlüssel hat.

### 7. Import: Zuordnung nur über den Fingerabdruck

Der Import liest die Datei als `serde_json::Value` und prüft von Hand, Eintrag für Eintrag:

1. Kein gültiges JSON, oder oben kein Objekt mit `antworten`-Liste → **ganze Datei abgewiesen**,
   nichts gelernt. Meldung: statische Ursache plus Zeile/Spalte (`serde_json::Error::line/column`),
   nie der Fehlertext von `serde_json`.
2. Ein Eintrag ist kein Objekt, hat nicht **genau** die Felder `fingerabdruck` und `kategorie` (beide
   Text), oder trägt ein weiteres Feld (`text`, `verwendungszweck`, `begruendung` …) → **Eintrag
   verworfen**. Ein Feld außer diesen beiden wird nie gelesen — auch nicht zur Zuordnung.
3. `kategorie` leer → Eintrag verworfen (eine leere Kategorie ist keine Antwort, wie in der
   `CategorizerBridge`).
4. `fingerabdruck` (getrimmt) ist nicht **exakt** gleich einem Fingerabdruck, den eine Auswertung
   ausgegeben hat (`window_evaluation_fingerprints`) → Eintrag verworfen. Kein Vergleich ohne
   Groß/klein, kein Präfix, keine Ähnlichkeit, keine Suche über Text. Dafür bekommt
   `SqliteCategoryRegistry` eine Lese-Methode `issued_among(&[String])`.
5. Der Rest geht in **einem** Aufruf an `SqliteCategoryRegistry::learn` — alles oder nichts.
   Derselbe Fingerabdruck mit zwei verschiedenen Etiketten → **ganze Datei abgewiesen**, mit der
   Position aus der Datei (das Register rät nicht, welches gemeint war). `learn`s eigene Prüfung auf
   `UnknownFingerprint` bleibt das letzte Netz.

Ein schon bekannter Fingerabdruck darf im Import stehen: dann ist es eine **Korrektur**, `learn`
ersetzt das Etikett. Der Lauf schreibt das Register nie; eine Korrektur bleibt also in jedem
weiteren Lauf erhalten.

### 8. Geplante Tests (alle Daten erfunden)

In `crates/io` gegen echtes SQLite, über `io::run::run` mit einer eigenen Fixture-CSV:

- **Leak-Test.** Die CSV trägt erfundene IBANs (Gegenkonto **und** im Verwendungszweck, kompakt und in
  Vierergruppen), eine erfundene BIC (Spalte **und** im Zweck, mit und ohne `BIC:`), einen Empfänger
  ohne Gläubiger-ID mit Klarnamen, denselben Namen im Zweck einer Firma, einen Namen aus
  `--personen`, eine Kundennummer, eine E-Mail-Adresse. Der Seed ist ein auffälliger Wert. Geprüft
  wird auf den **rohen Bytes** der Export-Datei: keine IBAN (kompakt, gruppiert, klein), keine BIC,
  kein Name (beliebige Schreibweise), keine der Nummern, keine E-Mail, kein `KONTO_`, nicht der Seed
  (dezimal und hex). Dazu: jeder Eintrag hat genau `fingerabdruck` und `text`.
- **Durchleiter.** Eine PayPal-Zeile: der Text ist der Zweck, nicht „PayPal …".
- **Nur Unbekanntes.** Ein Fingerabdruck, den das Register schon kennt, steht nicht im Export.
- **Round-Trip.** Lauf mit Export → kanonische Fixture-Antwort (`crates/io/fixtures/kategorien/`) →
  `kategorien-import` → `categories_of` liefert die Etiketten → zweiter Lauf: die gelernten Arten
  stehen nicht mehr im Export.
- **Korrektur bleibt.** Import Etikett A, Import Etikett B für denselben Fingerabdruck, dann ein Lauf
  → B.
- **Keine Text-Zuordnung.** Antworten mit (a) dem Firmennamen bzw. dem Text der Anfrage als
  `fingerabdruck`, (b) einem Fingerabdruck in anderer Groß-/Kleinschreibung, (c) `text` +
  `kategorie` ohne `fingerabdruck`, (d) einem Zusatzfeld → alle verworfen, nichts davon gelernt;
  die korrekten Einträge daneben werden gelernt. Die Meldung nennt keinen der Werte.
- **Widerspruch und kaputtes JSON** → nichts gelernt, Meldung redigiert.
- **Exit 1** → keine Export-Datei.

In `auswerten_specs` über die `EvaluationTestApi`: die Textwahl aus § 4 (Durchleiter, Firma mit und
ohne Name, Wort-Filter, Person nach § 9). In `crates/lauf`: die neue Option und der Unterbefehl.

### 9. Offene Entscheidungen für das Review

1. **Personen exportieren?** Empfehlung: **nein**. Eine Person (keine Gläubiger-ID) bekommt keine
   Anfrage. Grund: ihr Fingerabdruck ist `PERSON:PERSON_<hex>`, und das Pseudonym ist nur so stark wie
   der Seed. Der Seed ist ein frei gewählter `u64` ohne Mindestanforderung (`config/seed.rs`,
   `0` ist gültig). Bei einem schwachen Seed (`seed = 42`) kann der Chat-Anbieter mit einer Namensliste
   und den kleinen Seeds durchprobieren und den Namen zurückgewinnen. Dazu ist der Zweck einer
   Überweisung an eine Person der Text mit dem höchsten Risiko (Vornamen, Teilnamen — die
   bekannte Lücke aus 0010). Ohne Personen verlässt **kein** vom Seed abgeleiteter Wert die Maschine.
   Preis: Überweisungen an Personen (Miete an einen privaten Vermieter, Taschengeld) bekommen per Chat
   kein Etikett. Alternative: Personen mit Zweck (nach Wort-Filter) exportieren und eine
   Seed-Mindeststärke als eigenes Ticket fordern.
2. **Firma: nur der Name?** Empfehlung: ja (§ 4). Alternative: Name + gefilterter Zweck wie heute.
3. **Zusatzfeld im Import:** Empfehlung: Eintrag verwerfen (§ 7, Punkt 2). Alternative: Zusatzfelder
   still übergehen — LLMs hängen gern eine `begruendung` an.

**Bleibt als Grenze, auch nach dem Bau** (ehrlich benannt, keine Anonymisierung):

- Eine **natürliche Person mit Gläubiger-ID** (privater Vermieter, Verein, Einzelunternehmer) gilt als
  Firma; ihr Name geht als `text` hinaus. Abhilfe: ihren Namen vor dem ersten Lauf in
  `personen.toml` eintragen — dann wird er auch im Firmennamen zu `[PERSON]`.
- Ein Name, der weder Empfänger ohne Gläubiger-ID war noch in `personen.toml` steht, bleibt im Text
  und im Händler-Token stehen.
- Die Export-Datei ist lesbar. Der Nutzer trägt sie selbst in den Chat — **vorher ansehen** ist die
  letzte Schranke, und der Hilfetext sagt das.
