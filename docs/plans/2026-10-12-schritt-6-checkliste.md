# Schritt 6 — Checkliste für den echten Betrieb

Stand: 2026-10-12. Gehört zum [Abschlussplan](2026-10-12-1343-abschlussplan.md), Schritt 6.
Die Schritte 0–5 sind gebaut. Was jetzt fehlt, kann nur Markus tun: echte Daten, echte Prüfung.

**Grundregel:** Echte Daten liegen **nie** im Repo. Alles unten passiert in einem eigenen Ordner
außerhalb des Repos: `~/bank-auswertung-daten`. Nur `eigene-konten.toml` und `personen.toml` sind
im Repo per `.gitignore` geschützt — die Datenbank, die CSVs, der Seed, der Bericht und die
Chat-Dateien sind es **nicht**. Darum der eigene Ordner.

Pfad zum Repo in den Befehlen: `REPO="/Volumes/Lexar 790/ext_home/miorio/Documents/code/python/bank_auswertung"`.

---

## 1. Programm bauen und Ordner anlegen

```bash
REPO="/Volumes/Lexar 790/ext_home/miorio/Documents/code/python/bank_auswertung"
cd "$REPO" && cargo build --release -p lauf
mkdir -p ~/bank-auswertung-daten && chmod 700 ~/bank-auswertung-daten
cp "$REPO/target/release/bank-auswertung" ~/bank-auswertung-daten/
cd ~/bank-auswertung-daten
```

Fertig, wenn: `./bank-auswertung` gibt den Hilfetext aus (Exit 2 ist dabei normal).

## 2. Die fünf Config-Dateien anlegen

```bash
cd ~/bank-auswertung-daten
cp "$REPO/eigene-konten.example.toml" eigene-konten.toml
cp "$REPO/personen.example.toml"      personen.toml
cp "$REPO/sparen.example.toml"        sparen.toml
printf 'window_days = 400\n' > fenster.toml
printf 'seed = %s\n' "$(python3 -c 'import secrets; print(secrets.randbelow(2**63 - 1) + 1)')" > seed.toml
chmod 600 eigene-konten.toml personen.toml seed.toml
```

Dann von Hand ausfüllen:

1. **`eigene-konten.toml`** (Ticket 0002): die vier Platzhalter-IBANs durch die echten ersetzen —
   Giro, Haushalt, Spar, Vorlauf. Der Loader lehnt Tippfehler ab (Prüfziffer).
2. **`personen.toml`**: alle Namen eintragen, die im Verwendungszweck auftauchen können —
   **auch** Vornamen und Kurzformen, PayPal-Freunde, den eigenen Namen und Personen mit eigener
   Gläubiger-ID (privater Vermieter, Verein). **Vor dem ersten Lauf** — spätere Namen wirken nicht
   rückwirkend auf schon eingelesene Zeilen.
3. **`sparen.toml`**: die Schwelle in Cent setzen (`10000` = 100,00 € im Jahr).
4. **`seed.toml`**: nichts tun, aber **sichern** (z. B. im Passwort-Manager) und **nie ändern**.
   Ein neuer Seed macht alle Pseudonyme neu; die alte Datenbank passt dann nicht mehr dazu.

## 3. Kontoauszüge exportieren

Im Online-Banking je Konto einen **CSV-CAMT-v8**-Export ziehen, so weit zurück wie möglich, und
ablegen als:

```text
~/bank-auswertung-daten/giro.csv
~/bank-auswertung-daten/haushalt.csv
~/bank-auswertung-daten/spar.csv
~/bank-auswertung-daten/vorlauf.csv
```

Wichtig: Jeder Export endet an einem **vollen Tag** (kein Tag halb drin). Sonst können Buchungen
dieses Tages bei einem späteren, überlappenden Export als „schon bekannt“ verloren gehen (ADR-0016).

## 4. Vier Läufe in eine Datenbank

Ein Konto je Lauf, alle in dieselbe `auswertung.sqlite`. Nach jedem Befehl `echo $?` — es muss
`0` dastehen.

```bash
cd ~/bank-auswertung-daten
for konto in giro haushalt spar vorlauf; do
  ./bank-auswertung --db auswertung.sqlite --csv "$konto.csv" \
    --fenster fenster.toml --seed seed.toml --eigene-konten eigene-konten.toml \
    --personen personen.toml --sparen sparen.toml --bericht bericht.html \
    --kategorien-export kategorien-anfrage.json
  echo "$konto: Exit $?"
done
open bericht.html
```

Jeder Lauf wertet **alle** Konten der Datenbank aus; maßgeblich ist der Bericht des letzten Laufs.

Prüfen:

- Alle vier Zeilen zeigen `Exit 0`.
- Oben im Bericht steht **kein** Hinweis „Analyse unvollständig“. Steht er da: ADR-0017 lesen
  (eine Buchung ist ausgesondert; frische `--db` oder Redrive per SQL).
- Der Cashflow zeigt alle vier Rollen.

Exit 1 heißt: eine Nachricht ist gescheitert, der Grund steht im Lauf-Protokoll (Fehlerkanal).
Exit 2 heißt: ein Tippfehler in der Befehlszeile.

## 5. Gebühren-Marker prüfen (Ticket 0006)

Das Programm erkennt Gebühren am Buchungstext, wenn er `ENTGELT`, `GEBUEHR` oder `GEBÜHR` enthält
(`crates/auswerten_domain/src/value_objects/booking_kind.rs`).

1. In den vier CSVs die Spalte `Buchungstext` aller Zeilen ansehen, die echte Bankgebühren sind.
2. Mit dem Abschnitt „Gebühren“ im Bericht vergleichen: fehlt eine, oder ist etwas drin, das keine
   Gebühr ist?
3. Das Ergebnis in `docs/issues/0006-gebuehren-klassifikation.md` notieren (nur die
   **Buchungstexte**, keine Beträge, keine IBANs). Fehlt ein Marker, wird er als Folge-Ticket
   nachgezogen.

## 6. Kategorien per Chat

1. `kategorien-anfrage.json` **erst ansehen**. Darin stehen nur Gläubiger-IDs, Firmennamen und bei
   Zahlungsdiensten der Verwendungszweck ohne Wörter mit Ziffer oder `@`. Personen und
   Kartenzahlungen fehlen bewusst. Steht ein Name darin, der nicht hinaus soll: in `personen.toml`
   eintragen und mit frischer `--db` neu beginnen (Schritt 4).
2. Den ganzen Inhalt in einen Chat geben. Die Anleitung steht in der Datei selbst.
3. Die Antwort als `kategorien-antwort.json` speichern, Form:
   `{"antworten": [{"fingerabdruck": "…", "kategorie": "…"}]}`.
4. Importieren und den letzten Lauf wiederholen, damit der Bericht die Kategorien zeigt:

```bash
cd ~/bank-auswertung-daten
./bank-auswertung kategorien-import --db auswertung.sqlite --antwort kategorien-antwort.json
echo "Import: Exit $?"
./bank-auswertung --db auswertung.sqlite --csv vorlauf.csv \
  --fenster fenster.toml --seed seed.toml --eigene-konten eigene-konten.toml \
  --personen personen.toml --sparen sparen.toml --bericht bericht.html
echo "Lauf: Exit $?"
```

Ein wiederholter Lauf mit derselben CSV ergibt dieselben Zahlen; er zieht nur die Kategorien neu.

## 7. Review-Sitzung (Ticket 0013)

Gemeinsam mit Claude den Bericht durchgehen. Fertig, wenn:

1. **Eine Korrektur gelernt ist:** in `kategorien-antwort.json` bei einem Eintrag die Kategorie
   ändern, als `kategorien-korrektur.json` speichern und mit `--korrektur` importieren — dann den
   Lauf aus Schritt 6 wiederholen und sehen, dass die neue Kategorie im Bericht steht:

   ```bash
   ./bank-auswertung kategorien-import --db auswertung.sqlite --antwort kategorien-korrektur.json --korrektur
   ```

2. **Ein Sparpotenzial benannt ist:** ein Treiber aus „Spar-Kandidaten“ mit geschätzter Ersparnis
   im Jahr.
3. **Beobachtungen festgehalten sind:** in `docs/issues/0013-…md` (ohne echte Namen, IBANs oder
   Beträge, die jemanden identifizieren).

## 8. Abschließen

- Ticket 0002: AK „echte IBANs erfasst“ abhaken, schließen (lokal + GitHub #93).
- Ticket 0006: Ergebnis der Marker-Prüfung eintragen.
- Ticket 0013: AK abhaken, schließen (lokal + GitHub #100).
- Damit sind alle sieben Punkte der Definition „Projekt fertig“ erfüllt.

## Wenn etwas schiefgeht

| Zeichen | Was tun |
|---|---|
| Exit 2 | Befehlszeile prüfen; `./bank-auswertung` ohne Optionen zeigt die Hilfe. |
| „Schema-Version“ im Fehler | Die Datenbank ist von einem anderen Programmstand: frische `--db`. |
| „Datei aus älterem Stand, frische --db verwenden“ | Genau das tun. |
| „Analyse unvollständig“ | ADR-0017 lesen; meist frische `--db`. |
| Ein Name steht im Bericht oder im Export | In `personen.toml` eintragen, mit frischer `--db` neu beginnen. |
