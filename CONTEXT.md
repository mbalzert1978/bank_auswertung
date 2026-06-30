# Bank-Auswertung

Liest Kontoumsätze aus Bank-Exportformaten ein und hilft dabei, wiederkehrende
Kostentreiber und Sparpotenziale zu finden. Reine Analyse — löst keine
Transaktionen aus, ändert keine Kontodaten.

## Language

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
Empfänger-Namen festgemacht werden.
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

### Flagged ambiguities

**Firma vs. Privatperson** — entscheidet, ob der Empfänger-Name im Klartext
bleibt (Firma) oder pseudonymisiert wird (Privatperson). Auflösung: heuristisch
über die Gläubiger-ID (vorhanden → Firma; fehlt → Privatperson). Die Heuristik
ist bewusst **fail-safe**: bei Unsicherheit als Privatperson behandeln und
pseudonymisieren, weil der teurere Fehler der Klartext-Leak eines echten Namens
ist, nicht ein Pseudonym zu viel.

## Example dialogue

> **Dev:** Diese Buchung — `EDEKA SAGT DANKE`, 42 € — bekommt die eine Kategorie?
> **Markus:** Ja, „Lebensmittel". Den Namen erkennst du, das Programm nicht.
> **Dev:** Und die 12,99 € an `PERSON_07` jeden Monat?
> **Markus:** Das ist ein wiederkehrender Abfluss. Wer `PERSON_07` ist, sehe nur
> ich lokal — dir reicht, dass es jeden Monat fließt.
> **Dev:** Die 4,90 € mit Buchungstext `ENTGELT`?
> **Markus:** Gebühr der Bank. Die will ich extra ausgewiesen haben.
