---
id: "0047"
title: Gläubiger-IDs der Zahlungsintermediäre aus echten Auszugsdaten ergänzen
status: closed
---

# Gläubiger-IDs der Zahlungsintermediäre aus echten Auszugsdaten ergänzen

**Typ:** HITL

## Parent

Folge aus [0041](0041-fingerabdruck-als-domaenenfunktion.md) — dort entstand die
Intermediär-Erkennung des `Fingerabdruck`s, mit der hier zu schließenden Lücke.

## What to build

Die **Erkennung der Zahlungsintermediäre** vollständig machen — und dabei entscheiden, wo das Wissen
über sie am Ende lebt.

Die Erkennung schlüsselt seit [0041](0041-fingerabdruck-als-domaenenfunktion.md) vorrangig auf die
**Gläubiger-ID**, weil sie an einer Firma Pflichtfeld ist, während der Empfängername von `einlesen`
an durchweg optional ist. Hinterlegt ist bislang aber **nur PayPal**; für Klarna und Amazon Payments
trägt allein der Namens-Marker. Bei einer Buchung, die
diese beiden **ohne geführten Namen** ausweist, fällt die Erkennung darum **still** aus: der
Durchleiter verklumpt seine Händler wieder zu **einer** Buchungsart. Nichts schlägt fehl — das
Ergebnis ist nur falsch, und die Wiederkehr-Erkennung hängt am falschen Schlüssel.

Zwei Teile, die zusammengehören:

1. **Die Daten beschaffen.** Die Gläubiger-IDs (SEPA Creditor Identifier) von **Klarna** und
   **Amazon Payments** aus einem echten Kontoauszug ablesen und die für PayPal hinterlegte ID gegen
   dieselbe Quelle **verifizieren** — sie wurde in 0041 aus allgemeiner Kenntnis gesetzt, nicht aus
   Auszugsdaten. Das ist der HITL-Anteil: eine falsche ID ist schlimmer als eine fehlende, weil sie
   stumm die falsche Firma als Durchleiter behandelt.
2. **Die Ablage entscheiden.** Bleibt die Liste eine **Domänen-Konstante** (konsistent mit
   [ADR-0004](../adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md): dass ein Durchleiter den
   Händler verdeckt, ist eine Aussage über die Analyse, nicht über die Umgebung), oder wandert sie in
   eine **Konfiguration** analog zur Eigene-Konten-Config aus
   [0042](0042-eigene-konten-config-und-gegenkonto-auf-token.md)? Die Konfig-Variante ist ohne
   Rebuild erweiterbar, kostet aber einen Port samt io-Adapter und verschiebt eine
   Analyse-Entscheidung nach außen. Die Entscheidung ist bewusst offen und Teil dieses Tickets; sie
   gehört mit ihrer Begründung dokumentiert.

Unberührt bleibt die **Bildung** des Händler-Tokens — die ist gebaut und getestet. Hier geht es
allein um das Erkennen des Durchleiters.

## Acceptance criteria

- [x] Die Gläubiger-IDs von Klarna und Amazon Payments stammen **aus einem echten Auszug**, nicht aus
      allgemeiner Kenntnis; die Herkunft ist nachvollziehbar festgehalten. *(Amazon: erfüllt, s.
      Umsetzung. Klarna: bewusst entfallen — der Dienst wird nicht genutzt, es existiert keine echte
      Quelle; eine ID aus allgemeiner Kenntnis wäre genau der Fehler, den dieses Ticket verhindert.)*
- [x] Die für PayPal hinterlegte ID ist gegen dieselbe Quelle verifiziert oder korrigiert.
      *(Verifiziert: identisch mit der in 0041 gesetzten `LU96ZZZ0000000000000000058`.)*
- [x] Eine Buchung über Klarna bzw. Amazon Payments **ohne geführten Empfängernamen** trennt ihre
      Händler — belegt durch je einen Black-Box-Spec über die Fluent-TestApi des Slice, analog zu dem
      aus 0041 für den namenlosen Durchleiter. *(Amazon: zwei Specs, s. Umsetzung. Klarna: entfällt
      mit der ID, s. o. — der Namens-Marker bleibt als Auffangsignal.)*
- [x] Die Ablage-Entscheidung (Domänen-Konstante vs. Konfiguration) ist getroffen und **mit ihrer
      Begründung dokumentiert** — bei Konfiguration als ADR, weil sie die ADR-0004-Linie berührt.
      *(Entscheidung: Domänen-Konstante, s. Umsetzung — darum kein ADR.)*
- [x] Die Erkennung greift weiterhin **vorrangig** auf das Pflichtfeld Gläubiger-ID; der Name bleibt
      nachgelagertes Auffangsignal.
- [x] Der Unvollständigkeits-Hinweis, den 0041 an der Durchleiter-Liste hinterlassen hat, ist
      entfernt oder auf den dann tatsächlich verbleibenden Rest eingekürzt. *(Eingekürzt auf die
      Rest-Lücke Klarna, mit Begründung an der Konstante.)*
- [x] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Umsetzung (2026-07-20)

**Herkunft der IDs (HITL).** Alle Werte hat der Nutzer am 20.07.2026 aus den Detailansichten
eigener Lastschrift-Buchungen (Online-Banking/Kontoauszug) abgelesen und in der Session
durchgegeben — keine ID stammt aus allgemeiner Kenntnis:

| Gläubiger-ID | Firmierung laut Auszug | Rolle |
| --- | --- | --- |
| `LU96ZZZ0000000000000000058` | PayPal Europe S.a.r.l. et Cie S.C.A. | verifiziert (0041-Wert bestätigt) |
| `DE94ZZZ00000561653` | AMAZON PAYMENTS EUROPE S.C.A. | neu — Einzieher der Marketplace-Käufe |
| `DE96ZZZ00000594888` | AMAZON DIGITAL GERMANY GMBH | neu — Prime/Kindle & Co. |
| `DE24ZZZ00000561652` | AMAZON EU S.A R.L., NIEDERLASSUNG DEUTSCHLAND | neu — Direkteinkauf |

**Zuschnitts-Änderungen gegenüber dem Ticket-Text** (beide vom Nutzer in der Session entschieden):

1. **Klarna entfällt.** Der Dienst wird nicht genutzt; echte Quelldaten existieren nicht und sind
   auch nicht zu erwarten. Eine erfundene ID behandelte still die falsche Firma als Durchleiter —
   die Rest-Lücke ist darum als solche an der Konstante dokumentiert, der Namens-Marker bleibt als
   Auffangsignal. Nachtragen, sobald Klarna in echten Auszügen auftaucht.
2. **Amazon weitet sich auf drei Firmierungen.** Der echte Auszug zeigt Amazon nicht als *eine*
   Firma „Amazon Payments", sondern als drei Einzieher (Payments/Digital/EU). Bewusste
   **Analyse-Entscheidung des Nutzers**: auch die Direkt-Firmierungen werden als Durchleiter
   behandelt und ihre Buchungen nach dem Verwendungszweck-Token getrennt (Prime vs. Kindle vs.
   Einkauf) — der Zweck trägt dort die *Art*, die Firmierung nur den Kanal.

**Ablage-Entscheidung: Domänen-Konstante** (kein ADR nötig, da keine Konfiguration):

- Konsistent mit [ADR-0004](../adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md): dass ein
  Durchleiter den Händler verdeckt — und dass Amazons Zweck die Art trägt — ist eine Aussage über
  die **Analyse**, nicht über die Umgebung. Genau solche Aussagen leben in der Domäne.
- Die Liste ändert sich nur mit **neuer Evidenz aus echten Auszügen** (selten, HITL, review-würdig);
  ein Rebuild ist für ein lokales Ein-Nutzer-Werkzeug der passende Änderungsweg.
- Die Konfig-Variante kostete Port + io-Adapter und verschöbe eine Analyse-Entscheidung nach außen,
  ohne dass es einen Nutzer gäbe, der die Liste zur Laufzeit erweitern muss (YAGNI).

**Specs** (Black-Box über die Fluent-TestApi, `crates/auswerten_specs/tests/fingerabdruck.rs` —
die Fingerabdruck-Specs sind dorthin ausgegliedert, weil `auswerten.rs` die 1000-Zeilen-Schwelle
überschritten hatte):
`amazon_payments_ohne_gefuehrten_namen_verklumpt_seine_haendler_nicht` und
`auch_amazon_digital_und_amazon_eu_tragen_den_haendler_token_ihres_zwecks` — beide test-first
(rot gegen den alten Stand, grün mit den IDs).

## Blocked by

Blocked by [0041](0041-fingerabdruck-als-domaenenfunktion.md) — die Intermediär-Erkennung, die hier
vervollständigt wird, entsteht dort.

Fachlich zusätzlich abhängig von der offenen Vorbedingung **echte CSV-CAMT-v8-Beispieldatei**
(README „Offen"): ohne echten Auszug sind die Gläubiger-IDs nicht zu beschaffen.
