# Bank-Auswertung

Liest Kontoumsätze aus Bank-Exportformaten ein und hilft dabei, wiederkehrende
Kostentreiber und Sparpotenziale zu finden. Reine Analyse — löst keine
Transaktionen aus, ändert keine Kontodaten.

## Sprache

> **Code-Bezeichner-Regel:** Die hier definierten Begriffe sind das
> Domänenvokabular (UBQL) und benennen die **Sprache** — Doc-Comments,
> Testnamen/-helfer und `expect`-Meldungen bleiben **deutsch**. Der **Code**
> dagegen ist durchgängig **englisch**: Typen, Felder, Varianten und Methoden
> tragen englische Namen, die den deutschen Begriff übersetzen (im einlesen-Slice
> z. B. „Empfänger" → `Recipient`, „Betrag" → `Amount`, „Richtung" → `Direction`
> mit `Incoming`/`Outgoing`, „Verwendungszweck" → `Purpose`, „Buchungstext" →
> `BookingText`, „Gläubiger-ID" → `CreditorId`, „Rohzeile" → `RawLine`; im fenstern-Slice
> „Auswertungsfenster" → `AnalysisWindow`, „fenstern" → `Windowing`). Kein
> Deutsch im Code außer in Doc-Comments und Tests. Ausführlich im Abschnitt
> „Language convention" von [CLAUDE.md](CLAUDE.md).

**Buchung**:
Ein einzelner Kontoumsatz (eine Zeile im Export) mit Betrag, Datum, Richtung
(Ein-/Ausgang) und beschreibenden Feldern.
_Avoid_: Transaktion (zu nah an „löst etwas aus"), Posten

**Empfänger**:
Die Gegenpartei einer Buchung — bei Ausgaben der Zahlungsempfänger, bei
Eingängen der Auftraggeber. Trägt das wichtigste Signal für die Kategorie.
_Avoid_: Zahlungsbeteiligter, Kontrahent

**Verwendungszweck**:
Der Freitext einer Buchung. Zweitwichtigstes Kategorie-Signal nach dem Empfänger.
_Avoid_: Buchungstext (ist ein eigenes Feld!), Betreff, Referenz

**Buchungstext**:
Die maschinelle Art der Buchung (`LASTSCHRIFT`, `DAUERAUFTRAG`, `ÜBERWEISUNG`,
`ENTGELT` …). Treiber der deterministischen Wiederholungs- und Gebühren-Erkennung.
_Avoid_: Verwendungszweck (anderes Feld), Typ

**Gläubiger-ID**:
Eindeutige Kennung der Firma, die per Lastschrift einzieht (SEPA Creditor ID).
Identifiziert die **Firma**, nicht dich — Anker für die Abo-Erkennung.
_Avoid_: Mandatsreferenz (verwandt, aber pro Mandat statt pro Gläubiger)

**Kategorie**:
Die semantische Sammelschublade einer Buchung (z. B. „Lebensmittel",
„Spiel & Spaß", „Mobilität"). Wird vom LLM anhand von Empfänger +
Verwendungszweck vergeben, nicht vom Programm.
_Avoid_: Tag, Label, Gruppe

**Zahlungsintermediär**:
Ein Empfänger, der Zahlungen nur durchleitet (PayPal, Klarna, Amazon Payments).
Der echte Händler — und damit die Kategorie — steckt hier **nicht** im Empfänger,
sondern im Verwendungszweck. Kategorisierung darf bei Intermediären nicht am
Empfänger-Namen festgemacht werden. Seit Issue 0047 werden auch Amazons
Direkt-Firmierungen (Digital/EU) bewusst so behandelt — eine Analyse-Entscheidung:
ihr Verwendungszweck trägt die _Art_ (Prime vs. Kindle vs. Einkauf), die
Firmierung nur den Kanal.
_Avoid_: Zahlungsdienstleister (zu breit)

**Wiederkehrender Abfluss**:
Eine Ausgabe, die in festem Rhythmus mit ähnlichem Betrag und gleichem
**Fingerabdruck** (bzw. gleicher Gläubiger-ID, wo vorhanden) auftritt (Abos,
Daueraufträge) — der Fingerabdruck, nicht der rohe Empfänger, ist der Schlüssel,
damit ein Zahlungsintermediär nicht alle Händler zu einem Klumpen zusammenfasst.
Vom Programm deterministisch erkannt — das eigentliche Untersuchungsziel.
_Avoid_: Abo (Spezialfall), wiederkehrende Zahlung

**Gebühr**:
Eine Buchung, mit der die Bank selbst Geld einzieht (Kontoführung, Entgelte),
erkennbar am Buchungstext `ENTGELT` o. Ä.
_Avoid_: Kosten (zu allgemein)

**Umbuchung**:
Eine Buchung, die Geld zwischen den **eigenen** Konten verschiebt (Gegen-IBAN
ist eine der eigenen). Kein Abfluss, kein verlorenes Geld — wird aus der
„Wo bluten wir"-Analyse ausgeschlossen, bleibt aber sichtbar.
_Avoid_: Eigenübertragung, Umparken

**Umbuchungskosten**:
Die Gebühr, die die Bank für eine Umbuchung berechnet. Anders als die Umbuchung
selbst ist das echter Abfluss (eine **Gebühr**) — wegen ihr wurde das
Mehrkontenmodell zusammengestrichen, also Hauptverdächtiger.

**Konto-Rolle**:
Die Funktion, die ein Konto im Mehrkontenmodell hat — hier: Giro (laufend),
Haushalt, Spar, Vorlauf (für anstehende Fixkosten reserviert). Beeinflusst die
Deutung: Zufluss aufs Spar-Konto ist Sparen, nicht Konsum.

**Obfuskierung**:
Das lokale Entfernen reiner Identifikatoren vor der Weitergabe an das LLM.
Strukturelle Identifikatoren (eigene IBAN/Kontonr., Gegen-IBAN, BIC) fallen
vollständig weg. Die semantischen Freitext-Felder (Empfänger, Verwendungszweck)
bleiben für die Kategorie erhalten, werden aber best-effort gescrubbt:
eingebettete Identifikatoren (IBANs, bekannte Personen-Token) werden entfernt —
heuristisch, nicht beweisbar vollständig.
_Avoid_: Anonymisierung (impliziert Vollständigkeit, die hier nicht gilt)

**Pseudonymisierung**:
Das konsistente Ersetzen von **Privatpersonen**-Namen durch ein stabiles Token
(`PERSON_07` → immer dieselbe Person, auch über Importe hinweg). Die
Klartext-Zuordnung bleibt lokal. Die Stabilität trägt die Wiederkehr-Erkennung
mit: bei Privatpersonen geht das Pseudonym in den **Fingerabdruck** ein, über den
Wiederkehr und Korrekturen schlüsseln — ein wechselndes Pseudonym bräche beides,
ohne preiszugeben, wer dahintersteckt.

**Fingerabdruck**:
Ein stabiler Schlüssel, der eine _Art_ von Buchung für die Kategorisierung
identifiziert — aus dem Empfänger, bei Zahlungsintermediären zusätzlich aus einem
Händler-Token des Verwendungszwecks. Die Domäne fragt das LLM nur für ihr noch
**unbekannte** Fingerabdrücke; gelernte Korrekturen schlüsseln auf ihn, und die
Deduplizierung über Fingerabdrücke hält den Chat-Payload klein.
_Avoid_: Hash, Schlüssel

**Auswertungsfenster**:
Ein materialisierter, **eingefrorener** Snapshot der obfuskierten Buchungen **einer**
paginierten Zeitscheibe (Seitengröße real ~400 Tage, bewusst **> 365**, damit ein voller
Jahreszyklus in **eine** Seite fällt). Die Einheit, über die die periodische Auswertung
(Cashflow/Gebühren/Pareto) **innerhalb** rechnet und aus deren Gesamtheit — über **alle**
Fenster — die **Wiederkehr** liest. Vom Schritt `fenstern` gebildet.
_Avoid_: Berichtszeitraum/Zeitraum (das Monats-Bucketing ist etwas anderes), Seite (bloße
Pager-Mechanik)

**fenstern**:
Der Pipeline-Schritt zwischen **Obfuskierung** und **Auswertung**, der die obfuskierte
Historie in begrenzte Zeitfenster **paginiert** und jedes als **Auswertungsfenster**
materialisiert (N→1). Benannt nach seiner Verantwortung — er _bildet_ das Fenster —, nicht
nach der Mechanik.
_Avoid_: sammeln/Collector (sammelt nichts von außen — die Daten liegen schon lokal),
Paginieren (nur die halbe Verantwortung)

### Markierte Mehrdeutigkeiten

**Firma vs. Privatperson** — entscheidet, ob der Empfänger-Name im Klartext
bleibt (Firma) oder pseudonymisiert wird (Privatperson). Auflösung: heuristisch
über die Gläubiger-ID (vorhanden → Firma; fehlt → Privatperson). Die Heuristik
ist bewusst **fail-safe**: bei Unsicherheit als Privatperson behandeln und
pseudonymisieren, weil der teurere Fehler der Klartext-Leak eines echten Namens
ist, nicht ein Pseudonym zu viel.

**Fenster vs. Zeitraum** — das **Auswertungsfenster** ist die **Seitengröße** des Pagers
(~400 Tage, bewusst > 1 Jahr, damit ein Jahreszyklus in eine Seite fällt), **nicht** der
Berichts-/Bucketing-**Zeitraum**. Der Bericht bucketet _innerhalb_ eines Fensters nach Monat.
Nicht verwechseln: ein Fenster deckt viele Zeiträume ab.

**Reichweite: pro-Fenster vs. fenster-übergreifend** — die periodische Auswertung
(Cashflow/Gebühren/Pareto) rechnet **innerhalb** eines Auswertungsfensters; die
**Wiederkehr** liest **über alle** Fenster hinweg (ein jährliches Abo erscheint je Seite nur
1×, also braucht es 2+ Vorkommen über die Seiten). Wiederkehr ist **nicht** pro-Fenster.

## Beispieldialog

> **Dev:** Diese Buchung — `EDEKA SAGT DANKE`, 42 € — bekommt die eine Kategorie?
> **Markus:** Ja, „Lebensmittel". Den Namen erkennst du, das Programm nicht.
> **Dev:** Und die 12,99 € an `PERSON_07` jeden Monat?
> **Markus:** Das ist ein wiederkehrender Abfluss. Wer `PERSON_07` ist, sehe nur
> ich lokal — dir reicht, dass es jeden Monat fließt.
> **Dev:** Die 4,90 € mit Buchungstext `ENTGELT`?
> **Markus:** Gebühr der Bank. Die will ich extra ausgewiesen haben.
