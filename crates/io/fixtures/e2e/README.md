# Vier-Konten-Nachweis — erfundene Fixtures

Ein Konto je Lauf, eine Datenbank für alle: vier CSV-CAMT-v8-Auszüge (`giro.csv`, `haushalt.csv`,
`spar.csv`, `vorlauf.csv`) plus die Configs, die ein Lauf braucht (`konten.toml`, `namen.toml`,
`fenster.toml`, `seed.toml`, `sparen.toml`). Gefahren werden sie im Test
`crates/io/src/run/e2e_tests.rs` und — als Nachweis von Hand — mit dem Binary `bank-auswertung`.

**Alles ist erfunden.** Keine Zeile stammt aus einem echten Auszug.

## Konvention

- **Eigene Konten:** IBANs an der Bundesbank-BLZ `100 000 00` mit erfundener Kontonummer und
  **gültiger** Prüfziffer (der Eigene-Konten-Loader prüft mod 97). Bei der Bundesbank führt keine
  Privatperson ein Konto — strukturell kein Privatkonto.
- **Gegenkonten:** IBANs mit Prüfziffer **`00`** — die gibt es real nicht. BICs erfunden
  (`FIKTDEFF…`).
- **Gläubiger-IDs:** Schema `DEkkZZZFIKTIV000nn` mit **gültiger** Prüfziffer (sonst deutet
  `obfuskieren` die Gegenpartei fail-safe als Person). Echte deutsche Kennungen sind rein numerisch;
  `FIKTIV` darin macht sie strukturell erfunden.
- **Namen und Texte:** frei erfunden, meist mit `FIKTIV` im Namen. Einzige Ausnahme ist das Wort
  `PAYPAL` in `PAYPAL FIKTIV ZAHLDIENST`: die Domäne erkennt einen Durchleiter an diesem
  Namens-Marker (`auswerten_domain`, `party.rs`). Seine Gläubiger-ID ist erfunden, nicht die echte.
- **Dateinamen:** `konten.toml` und `namen.toml` statt `eigene-konten.toml`/`personen.toml` — die
  beiden sind per `.gitignore` für die echten Dateien reserviert.

## Was die Daten zeigen sollen

| Berichts-Sektion | woher |
|---|---|
| Cashflow je Konto und Monat | alle vier Konten; Umbuchungen herausgerechnet |
| Umbuchungen | Giro → Haushalt, Spar, Vorlauf (Daueraufträge), Spar → Giro (25.05.2026) |
| Gebühren | `ENTGELTABSCHLUSS` monatlich auf Giro, quartalsweise auf Haushalt |
| Wiederkehr monatlich + Preissteigerung | `STREAMHAUS FIKTIV`: 12,99 € → 14,99 € ab 15.04.2026 |
| Wiederkehr jährlich | `HAUSRAT FIKTIV VERSICHERUNG AG` auf Vorlauf, 15.03.2025 und 16.03.2026 |
| Händler hinter einem Durchleiter | `MUSIKDIENST FIKTIV` und `CLOUDSPEICHER FIKTIV` über `PAYPAL FIKTIV ZAHLDIENST` |
| Firma mit Gläubiger-ID | `STROMWERK FIKTIV AG`, `FRISCHMARKT FIKTIV SE` u. a. |
| Person | `ERWIN BEISPIELMANN` (ohne Gläubiger-ID) |
| Vorgemerkte Zeile | Haushalt, 29.06.2026 — wird übersprungen und zählt nicht |
| Name aus `namen.toml` | `Tilda Erfunden` im Verwendungszweck (Durchleiter) und im Praxisnamen |

Die Datensätze sind so gewählt, dass sich Monatssummen von Hand nachrechnen lassen (der Test prüft
Giro Februar/Mai und Haushalt Juni auf den Cent).
