---
id: "0047"
title: Gläubiger-IDs der Zahlungsintermediäre aus echten Auszugsdaten ergänzen
status: blocked
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

- [ ] Die Gläubiger-IDs von Klarna und Amazon Payments stammen **aus einem echten Auszug**, nicht aus
      allgemeiner Kenntnis; die Herkunft ist nachvollziehbar festgehalten.
- [ ] Die für PayPal hinterlegte ID ist gegen dieselbe Quelle verifiziert oder korrigiert.
- [ ] Eine Buchung über Klarna bzw. Amazon Payments **ohne geführten Empfängernamen** trennt ihre
      Händler — belegt durch je einen Black-Box-Spec über die Fluent-TestApi des Slice, analog zu dem
      aus 0041 für den namenlosen Durchleiter.
- [ ] Die Ablage-Entscheidung (Domänen-Konstante vs. Konfiguration) ist getroffen und **mit ihrer
      Begründung dokumentiert** — bei Konfiguration als ADR, weil sie die ADR-0004-Linie berührt.
- [ ] Die Erkennung greift weiterhin **vorrangig** auf das Pflichtfeld Gläubiger-ID; der Name bleibt
      nachgelagertes Auffangsignal.
- [ ] Der Unvollständigkeits-Hinweis, den 0041 an der Durchleiter-Liste hinterlassen hat, ist
      entfernt oder auf den dann tatsächlich verbleibenden Rest eingekürzt.
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

Blocked by [0041](0041-fingerabdruck-als-domaenenfunktion.md) — die Intermediär-Erkennung, die hier
vervollständigt wird, entsteht dort.

Fachlich zusätzlich abhängig von der offenen Vorbedingung **echte CSV-CAMT-v8-Beispieldatei**
(README „Offen"): ohne echten Auszug sind die Gläubiger-IDs nicht zu beschaffen.
