# Abschlussplan: vom heutigen Stand zu „Projekt fertig"

Stand: 2026-10-12. Mit Markus abgestimmt; löst [2026-07-31-1335](2026-07-31-1335-aufwand-und-reihenfolge-der-offenen-posten.md)
als gültigen Reihenfolge-Plan ab (dessen Aufwandsschätzungen bleiben die Grundlage).

## Definition „Projekt fertig" (bestätigt)

1. **Ein Konto pro Lauf, eine Datenbank für alle.** Nach den Läufen für alle eigenen Konten
   (Giro, Haushalt, Spar, Vorlauf — je eine CSV) zeigt die Auswertung alle Konten gemeinsam. Ein Test
   belegt: derselbe Lauf mehrfach ergibt dieselben Zahlen.
2. **HTML-Bericht** mit: Cashflow je Konto und Monat (Umbuchungen herausgenettet), Umbuchungen
   daneben, Gebühren, Wiederkehr (monatlich, jährlich, Preissteigerung, Händler hinter
   Zahlungsdiensten getrennt), Pareto-Ranking, Spar-Kandidaten, Kategorie je Treiber („unbekannt"
   erlaubt).
3. **Kategorisierung per Chat:** Export nur unbekannter Fingerabdrücke, Import der Antwort; eine
   Korrektur bleibt im nächsten Lauf erhalten.
4. **Datenschutz:** Test belegt — kein IBAN, keine BIC, kein Klarname in Bericht oder Export;
   Freitext-Scrubbing für IBAN, BIC, Personen-Token.
5. **Betrieb:** Die Datenbank trägt eine Schema-Version (0084); eine vergiftete Nachricht
   blockiert keine späteren Läufe (0085).
6. **Die sechs Gates** sind grün.
7. **Mit Markus:** echte IBANs eingetragen (0002), Gebühren-Marker an echten Auszügen geprüft
   (0006), eine Review-Sitzung mit einer gelernten Korrektur und einem benannten Sparpotenzial (0013).

**Später / nicht im Abschluss:** 0086, 0087, XML-camt/MT940, API-Kategorisierer. 0008 wird nur
geschlossen (durch ADR-0010 und 0059 erledigt).

## Entscheidungen von Markus

| # | Frage | Antwort |
|---|---|---|
| V1 | Nicht committete `io`-Änderungen? | Committet als `00d329a` (Newtypes `SeedMarker`/`SourceKey`, Gates grün) — sie liegen in den Dateien, die Schritt 2 umbaut. |
| V2 | Zeilen-Schlüssel ohne echte Daten? | Ja: Form aus [docs/examples/csv_camt_v8_header.md](../examples/csv_camt_v8_header.md) und zwei Demozeilen. |
| V3 | Was ist ein realistischer Spar-Kandidat? | Alles, was über einem **Config-Wert** liegt. Das Programm schlägt vor, Markus entscheidet. |
| V4 | Welche Personen-Token scrubbt der Freitext? | **Automatisch** alle bekannten Empfänger-Namen aus `einlesen_lines.recipient_name` (liegen roh lokal, ADR-0008; eine reine Namensliste ist **keine** von ADR-0010 verbotene Klartext↔Token-Zuordnung) **plus** eine kleine lokale `personen.toml` (gitignored) für Namen, die nie Empfänger waren. Eingaben von Markus = Datei, Errechnetes = DB. |
| — | Zweiter Import desselben Kontos? | **Ergänzen**, nicht ersetzen. |
| — | Kommt `Umsatz vorgemerkt` vor? | Ja. |
| — | Eigene-Konten-Mapping? | Vier Konten laut [eigene-konten.example.toml](../../eigene-konten.example.toml); die echte `eigene-konten.toml` legt Markus erst in Schritt 6 lokal an. |

## Befunde, die der Plan auflöst

- **Tabellen-Schlüssel statt Duplikat-Suche.** Alle Tabellen haben nur eine generierte `id`
  (`crates/io/src/einlesen/store.rs:218`, `crates/io/src/fenstern/store.rs:214`); die
  fenster-übergreifende Abfrage scopt nur über das Datum (`crates/io/src/auswerten/source.rs:312`).
  Fenster früherer Läufe zählen deshalb erneut mit.
- **Vorgemerkte Zeilen brechen den Import.** Eine `Umsatz vorgemerkt`-Zeile hat ein leeres
  Valutadatum; `value_date` ist Pflichtfeld (`crates/einlesen_adapter/src/models/raw_record.rs:38`),
  also scheitert die **ganze** CSV. Sie trägt außerdem `0` als IBAN/BIC und keine Kundenreferenz.
- **Doku-Widersprüche** (in Schritt 0 zu korrigieren): CLAUDE.md und `crates/lauf/src/main.rs:62`
  verorten die Entdopplung in 0084 (stimmt nicht); `crates/io/src/lib.rs:16` behauptet einen
  Dead-Letter; PRD Z. 268–271 und Z. 409–411 sind veraltet; `docs/adr/README.md` endet bei 0012;
  ADR-0015 Z. 199–207 kündigt `sqlx` an, das ADR-0014 verworfen hat.

## Schritte

| Schritt | Inhalt | Issues/ADRs | läuft | Sitzungen |
|---|---|---|---|---|
| 0 | Aufräumen | 0008, 0009, 0084, Doku | zuerst | 0,5 |
| 1 | Schema-Version | 0084 | `io`, nacheinander | 1 |
| 2 | Zeilen-Schlüssel, lauf-eigene Fenster, Vorgemerktes überspringen | neues Ticket, ADR-0016 | `io`, nach 1 | 1–1,5 (Schätzung) |
| 3a | Wiederkehr | 0005 | parallel zu 1–2 | 1,5–2 |
| 3b | Scrubbing | 0010 | parallel zu 1–2 | 1 |
| 4a | Cashflow je Konto, Pareto, Spar-Kandidaten | 0007 | nach 2 + 3a | 1–1,5 |
| 4b | Kategorie-Register | 0009 | parallel zu 4a | 1 |
| 4c | Dead-Letter | 0085 | `io`, nach 2 | 0,5–1 |
| 5a | HTML-Bericht | 0012 | nach 4a | 1–1,5 |
| 5b | Chat-Export/-Import | 0011 | nach 3a, 3b, 4b | 1–1,5 |
| 6 | Echter Betrieb mit Markus | 0002, 0006, 0013 | zum Schluss | — |

Summe ≈ 10–13 Sitzungen.

### 0 — Aufräumen

- **Ziel:** Agenten arbeiten nicht gegen falsche Blocker.
- **Inhalt:** 0084 auf `open`, 0008 schließen, Blocker 0008 aus 0009 streichen; offene Tickets
  nach GitHub spiegeln (`.rules/common/tickets.md`); Doku-Widersprüche oben korrigieren; das neue
  Ticket für Schritt 2 anlegen (lokal + GitHub); PROGRESS.md nachziehen.
- **Fertig, wenn:** jeder Status stimmt mit dem Code überein und kein offenes Ticket ohne
  GitHub-Gegenstück ist.

### 1 — Schema-Version (0084)

- **Ziel:** eine Datenbank, die über viele Läufe und Programmversionen lebt, hat einen
  Migrations-Anker.
- **Fertig, wenn:** die AK von 0084 erfüllt sind. Schritt 2 ist dann die erste echte Migration.

### 2 — Zeilen-Schlüssel und lauf-eigene Fenster (neues Ticket, ADR-0016)

- **Ziel:** mehrere Läufe (je Konto, auch wiederholt) in **eine** Datenbank ergeben richtige Zahlen.
- **Entscheidung im ADR:**
  - Zeilen-Schlüssel = Auftragskonto + Kundenreferenz (End-to-End), wenn gefüllt; sonst Rückfall auf
    Auftragskonto + alle Spalten + laufende Nummer gleicher Zeilen. Grenze: ein Export darf keinen
    Tag mittendrin abschneiden.
  - Neuer Import **ergänzt**; eine schon bekannte Zeile wird nicht erneut angelegt.
  - Fenster und Auswertungen gehören zu genau einem Lauf; die fenster-übergreifende Abfrage scopt
    über den Lauf statt nur über das Datum.
  - `einlesen` überspringt `Umsatz vorgemerkt` **vor** der Pflichtfeld-Prüfung.
- **Fertig, wenn** (Test mit erfundenen Daten): A, dann B, dann A erneut ergibt dieselben Zahlen wie
  A+B einmal; zwei gleiche Buchungen am selben Tag zählen beide; eine vorgemerkte Zeile bricht nichts
  ab und zählt nicht. Gates grün.

### 3a — Wiederkehr (0005)

- **Fertig, wenn:** alle sechs AK von 0005 erfüllt sind.

### 3b — Scrubbing (0010)

- **Regel aus V4:** Namensliste = alle bekannten Empfänger-Namen aus der DB (neuer Lese-Seam auf
  `einlesen_lines`) plus `personen.toml` (gitignored, mit `.example`-Vorlage); fail-safe, im Zweifel
  ersetzen. Bekannte Kosten: Firmen ohne Gläubiger-ID gelten fail-safe als Person und werden mit
  geschwärzt.
- **Fertig, wenn:** die AK von 0010 erfüllt sind; die Doku nennt das Scrubbing ehrlich heuristisch.

### 4a — Cashflow je Konto, Pareto, Spar-Kandidaten (0007)

- **Regel aus V3:** Spar-Kandidat ist jeder Treiber über einem Config-Wert; das Programm schlägt vor,
  Markus entscheidet. Vorschlag zur Bezugsgröße: der **Jahresbetrag**, damit monatliche und jährliche
  Abos vergleichbar sind — beim Bau zu bestätigen.
- **Fertig, wenn:** die AK von 0007 erfüllt sind; Cashflow steht je Konto **und** Monat.

### 4b — Kategorie-Register (0009)

- **Fertig, wenn:** die AK von 0009 erfüllt sind (AK zur „Trennung vom Zuordnungs-Speicher" gelesen
  als „der `Speicher` trägt den Seed nicht").

### 4c — Dead-Letter (0085)

- **Form:** Status-Spalte in der Outbox (Standard, keine eigene Tabelle).
- **Fertig, wenn:** die AK von 0085 erfüllt sind.

### 5a — HTML-Bericht (0012)

- **Fertig, wenn:** die AK von 0012 erfüllt sind und der Inhalt Punkt 2 der Definition abdeckt;
  `assert_not_impl_any!` auf dem Bericht-DTO.

### 5b — Chat-Export/-Import (0011)

- **Vorher:** Datenschutz-Review — erster Datenabfluss nach außen.
- **Fertig, wenn:** die AK von 0011 erfüllt sind.

### 6 — Echter Betrieb mit Markus

- `eigene-konten.example.toml` → `eigene-konten.toml`, vier echte IBANs (lokal, gitignored).
- `personen.toml` für Namen anlegen, die nie Empfänger waren (lokal, gitignored).
- Vier Läufe, ein Konto je Lauf, in eine Datenbank.
- Gebühren-Marker gegen die echten Auszüge prüfen (0006).
- Review-Sitzung (0013).
- **Fertig, wenn:** alle sieben Punkte der Definition erfüllt sind.

## Parallelität

- Jeder Agent in einem eigenen Worktree unter `.claude/worktrees/`.
- `io`-Arbeit streng nacheinander: 1 → 2 → 4c (gemeinsam `crates/io/src/sqlite/mod.rs`).
- Parallel möglich: 3a + 3b neben 1–2; 4a + 4b (+ 4c); 5a + 5b.
- Nach jedem Schritt `/cargo-six-gates`.
