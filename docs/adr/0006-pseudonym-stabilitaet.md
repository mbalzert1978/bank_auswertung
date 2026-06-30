# Pseudonym-Stabilität: die Klartext-Zuordnung ist lokal **und persistent**

Der Fingerabdruck ist eine reine Funktion über dem Empfänger
([ADR-0004](0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)). Für eine
Privatperson (`Empfänger::Person(Pseudonym)`) geht das Pseudonym in den
Fingerabdruck ein — und der Fingerabdruck ist der Schlüssel für
Wiederkehr-Erkennung und gelernte Korrekturen
([ADR-0002](0002-dto-grenze-und-anti-corruption-layer.md)). Damit derselbe Mensch
über Importe hinweg denselben Fingerabdruck erhält, muss sein Pseudonym
**laufstabil** sein: `PERSON_07` heute ist `PERSON_07` beim nächsten
Monatsimport.

Ein pro Lauf vergebenes First-Seen-Zählwerk wäre nicht stabil — bei anderem
Eingabe-Set bekäme dieselbe Person eine andere Nummer, der Fingerabdruck ihres
Dauerauftrags bräche, und Wiederkehr-Erkennung wie gelernte Korrekturen
verfehlten genau die Person-zu-Person-Daueraufträge (z. B. die monatlichen
12,99 € an `PERSON_07` aus dem Beispieldialog).

Deshalb ist die Klartext↔Pseudonym-Zuordnung nicht nur lokal
([ADR-0001](0001-hexagonale-architektur.md)), sondern **persistent**: ein
lokaler Zuordnungs-Speicher des eingehenden Adapters, der den Rechner nie
verlässt, getrennt vom obfuskierten `Speicher`-Port. First-Seen-Vergabe, einmal
vergeben bleibt fest. Das liefert „`PERSON_07` → immer dieselbe Person" und
stabile, sprechende Etiketten zugleich.

Verworfene Alternative: ein deterministischer keyed Hash des rohen Identifikators
(stabil ohne Persistenz, lokal aus der Quelle rekonstruierbar). Tragfähig, aber
das sprechende Etikett (`PERSON_07`) bräuchte trotzdem eine eigene Abbildung — die
explizite Persistenz ist ehrlicher und einfacher als ein Hash plus separater
Etiketten-Schicht.

## Consequences

- Zwei getrennte lokale Speicher: der `Speicher`-Port hält nur obfuskierte DTOs
  (keine strukturellen Identifikatoren), der Zuordnungs-Speicher hält die
  Klartext-Abbildung und verlässt den Rechner nie. Die Trennung ist Absicht —
  Vermischung wäre ein Leck-Weg.
- Die Stabilität ist verbindlich, **bevor** Korrekturen persistiert werden
  (Schritt 4) — sonst schlüsseln gelernte Korrekturen auf instabile
  Fingerabdrücke.
- Eigene Konten (`KONTO_x`) sind ohnehin stabil: ihr Token kommt aus der
  Eigene-Konten-Config, nicht aus einer Lauf-Vergabe.
