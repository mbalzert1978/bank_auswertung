# Seed-basierte Pseudonym-Stabilität — die Stabilität ruht auf dem Seed, nicht auf einer persistenten Zuordnungstabelle

> **Status: maßgeblich.** Supersedet den **Kern** von
> [ADR-0006](0006-pseudonym-stabilitaet.md) (die persistente
> Klartext↔Pseudonym-Zuordnung). Steht im Kontext der
> [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)-Pipeline: `obfuskieren`
> ist ein **nachgelagerter Consumer**, kein Inline-Schritt des Eingangs.

[ADR-0006](0006-pseudonym-stabilitaet.md) hielt die Pseudonym-Stabilität an einer
**persistenten Klartext↔Pseudonym-Zuordnung** fest (First-Seen-Vergabe in einem
lokalen Zuordnungs-Speicher). Beim Bau des `obfuskieren`-Slice
([Plan 0016-1](../plans/0016-1-obfuskieren-slice-pipeline-consumer.md)) haben wir das
umgestellt: **die Stabilität ruht auf einem Seed, nicht auf einer Zuordnungstabelle.**

## Entscheidung

- **Deterministische Tokenvergabe.** Ein `Obfuscator`-Port (Random-Lib hinter
  Interface, io-erfüllt; Fake in den Specs) erzeugt aus **(Seed, Klartext)** ein Token.
  Gleicher Seed + gleiche Eingabe → gleiches Token (`KONTO_x`/`PERSON_x`) —
  **zustandslos**: **kein** First-Seen-Zähler, **keine** Klartext↔Token-Mapping-Tabelle,
  **kein** `ZuordnungsSpeicher`-Port.
- **Laufstabilität über den Seed.** Der Seed ist config-injiziert (Config-Seam,
  io-erfüllt) und wird **mit jedem obfuskierten Aggregat persistiert** (er fließt in die
  `obfuskiert`-Zeile, für Rehydrierung). Solange die Config denselben Seed liefert, ergibt
  derselbe Mensch über Importe hinweg dasselbe `PERSON_x` — und sein Fingerabdruck bleibt
  stabil (der Grund aus ADR-0006 bleibt der Grund; nur der Mechanismus wechselt).
- **`obfuskieren` liest per ID.** Der Slice ist ein Pipeline-Consumer: der Handler liest
  das rohe Aggregat **per ID** aus dem `einlesen`-Speicher (`RawBookingSource`-Port) — die
  Outbox-Message trägt nur die **ID** (Zeiger), die Nutzlast liegt in der Tabelle. (Nicht
  mehr das ADR-0006/ADR-0007-Bild „Obfuskierung inline am Eingangs-Adapter".)

Verhältnis zu ADR-0006: dessen **verworfene Alternative** — „ein deterministischer keyed
Hash des rohen Identifikators, stabil ohne Persistenz, lokal aus der Quelle
rekonstruierbar" — ist damit **der gewählte Weg**. Das sprechende Etikett (`PERSON_07`)
braucht keine eigene Abbildungs-Schicht: das Token-Schema liefert es selbst. `KONTO_x` war
schon in ADR-0006 stabil (aus der Eigene-Konten-Config) und bleibt es.

## Threat-Model — die relevante Grenze ist die Maschinen-Grenze

- Der Datenschutz hängt an der **Maschinen-Grenze** (LLM/Report), **nicht** an der
  Slice-Grenze. Der Seed, die `obfuskiert`-Tabelle **und** die rohe `einlesen`-Tabelle
  dürfen **lokal existieren und persistieren** (kein Unterschied zur lokal liegenden
  `.csv`; [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)).
- **Garantie:** die Egress-DTOs (`KategorieAnfrage` ans LLM, Report-Output) reichen
  **weder den Seed noch einen rohen Identifikator** weiter. Belegt durch den
  compilergeprüften „kein `Serialize`"-Guard (`assert_not_impl_any!`) auf dem Egress-DTO
  `ObfuscatedBooking`.
- **Vorgabe für den späteren `auswerten`-Slice:** der Seed ist ein lokales Geheimnis wie
  die Rohtabelle — er darf **nie** in ein Egress-DTO gelangen.

## Konsequenzen

- **Kein separater Zuordnungs-Speicher / `ZuordnungsSpeicher`-Port** mehr. Die
  ADR-0006-Regel „zwei getrennte lokale Speicher (Klartext↔Pseudonym-Mapping vs.
  obfuskierter `Speicher`)" wird ersetzt: das lokal zu haltende **Geheimnis ist der Seed**
  (plus die rohe `einlesen`-Tabelle); der obfuskierte `Speicher` und jedes Egress-DTO
  tragen **weder Seed noch Rohwert**. Der Anti-Leck-Kern („nichts Rohes/Geheimes verlässt
  die Maschine") bleibt — nur das Geheimnis ist jetzt der Seed statt einer
  Mapping-Tabelle.
- **Laufstabilität = die Config hält den Seed konstant.** Geht der Seed verloren oder
  ändert er sich, werden alle Token neu vergeben (die Analyse bleibt in sich konsistent,
  aber nicht import-übergreifend stabil) — bewusst akzeptiert und hier dokumentiert.
- **[Issue 0008](../issues/0008-persistente-pseudonym-zuordnung.md) (persistente
  Pseudonym-Zuordnung) ist als Mapping-Speicher-Slice hinfällig**; der Restbedarf ist nur
  noch die **Seed-Verwaltung in der Config**, kein eigener Slice.
- Freitext (`ObfuskierterText`) läuft weiter durch seinen Scrubbing-Engpass und wird
  **nicht** seed-tokenisiert; die **Gläubiger-ID** bleibt bewusst Klartext-Anker.
