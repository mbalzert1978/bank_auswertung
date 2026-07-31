# Aufwand und Reihenfolge der offenen Posten — Stand 2026-07-31

Übersicht über **alle** offenen Tickets mit Aufwandsschätzung und empfohlener Reihenfolge.
Kein Plan für ein einzelnes Issue (dafür die issue-nummerierten Dateien nebenan), sondern
die Landkarte darüber.

**Zeitstempel im Dateinamen mit Absicht:** spätere Fortschreibungen kommen als *neue* Datei
daneben, nicht als Änderung dieser hier. So kollidiert nichts, und jede Momentaufnahme
bleibt als das lesbar, was sie war.

**Stand, auf den sich alles bezieht:** `main` = `30a2382` (0083 gemergt, PR #92),
`dev` = `f5969d0` (drei Review-Befunde behoben). Sechs Gates grün.

## Wo das Projekt steht

Zwei Fragen, am Code verifiziert:

**Läuft alles lokal, ohne LLM?** Ja, und zwar *strukturell*. Die vollständige Fremd-Crate-Liste
des Workspace ist `chrono`, `csv`, `rusqlite` (bundled), `rust_decimal`, `rusty-money`, `serde`,
`siphasher`, `static_assertions`, `toml` — keine einzige netzwerkfähige. Kein `reqwest`, `hyper`,
`ureq`, `curl`, `tokio`, `rustls`, kein Socket. Das Orakel (`UnansweredCategorizer`) und das
Register (`EmptyCategoryRegistry`) sind echte Adapter mit bewusst **leerer** Antwort, damit ein
Lauf ohne LLM durchläuft statt daran zu scheitern (ADR-0004s Lackmustest). Die Maschinengrenze
hält also nicht durch Disziplin, sondern durch Abwesenheit der Fähigkeit. Der erste Egress
überhaupt entsteht erst mit Issue 0011.

*Vorbehalt:* „nur lokal" heißt nicht „alles pseudonymisiert". `einlesen_lines` liegt **vor** der
Privacy-Grenze und trägt die Rohdaten (IBAN im Klartext). Die `--db`-Datei ist damit so
schützenswert wie der Kontoauszug selbst.

**Steht die Domäne?** Das Fundament ja, die Auswertung nein. Nachgewiesen an einem echten Lauf:

| Was | Ergebnis | Zustand |
|---|---|---|
| Monats-Cashflow | 2026-06: ein 257000, aus 173060 (Minor Units) | rechnet |
| Gegenkonto Eigen/Fremd | `own\|spar`, `foreign` | rechnet, auf Tokens |
| Fingerabdrücke | 41 distinct, z. B. `COMPANY:DE00ZZZ00000000001` | rechnet |
| Kategorien | 0 Zeilen | Orakel antwortet leer (0011) |
| Vorkommen / Zeitachse | 49 berechnet — **und weggeworfen** | `CountingOccurrenceSink` zählt und verwirft; der Abnehmer ist 0005 |

Die vier Analyzer, für die das Projekt existiert, sind zu **0 von 4** gebaut. Wer heute seinen
echten Auszug durchschickt, bekommt eine korrekt befüllte Datenbank — aber keine Antwort auf
„wo geht mein Geld hin".

## Kalibrierung der Schätzung

Nicht geraten, sondern an der Historie dieses Repos gemessen:

| Bezug | Umfang |
|---|---|
| Projekt bisher | 30.06.–31.07.2026, **ein Monat**, 161 Commits auf `main`, ~72 Tickets geschlossen |
| Typischer Feature-PR | 5–19 Dateien, 300–1400 Zeilen (`0057`: 1060, `0058`: 1376, `0074`–`0077`: je 300–450) |
| Größter echter PR | `0083` + io-Teil von `0002`: 37 Dateien, 2042 Zeilen |
| Sitzung vom 31.07. | 3 Befunde behoben, verifiziert, dokumentiert: 12 Dateien, 255/93 Zeilen |

Die offenen Tickets sind auffällig gleich groß: 41–62 Zeilen Spezifikation, 4–6
Akzeptanzkriterien — dieselbe Bandbreite wie die bereits gelieferten.

**Einheit „Sitzung"** = eine fokussierte Arbeitseinheit inklusive der sechs Gates und einer
empirischen Gegenprobe. Bewusst *keine* Stundenangabe: die Streuung kommt aus Entscheidungen
und Review-Runden, nicht aus Tippgeschwindigkeit.

## Schätzung je Ticket

| Ticket | Sitzungen | Was den Aufwand treibt |
|---|---|---|
| 0004 Umbuchung-Netting | **1** | Reine Domänenlogik auf vorhandenen Daten — `own/foreign` wird schon berechnet |
| 0006 Gebühren-Klassifikation | **1** | Regeln auf `booking_text`/Fingerabdruck, mechanisch |
| 0005 Wiederkehr-Erkennung | **1,5–2** | Größte Spec; Intervall-Erkennung mit Toleranz ist Modellierung |
| 0007 Pareto / Spar-Kandidaten | **1–1,5** | Braucht 0004 vorher, sonst rankt es Umbuchungen als Kostentreiber |
| 0012 HTML-Bericht | **1–1,5** | Neuer Presenter, Egress-DTO, `Serialize`-Guard wird scharf |
| 0009 Speicher + Kategorie-Register | **1** | Folgt dem Muster der vier vorhandenen Stores |
| 0011 Kategorisierer-Chat-Adapter | **1–1,5** | **Erster Egress überhaupt** — Privacy-Review nicht abkürzbar |
| 0010 Scrubbing-Regeln | **1** | + Entscheidung nötig |
| 0084 Schema-Owner + Versionsstempel | **1** | `blocked` |
| 0085 Poison-Message / Dead-Letter | **0,5–1** | Erst nach der Entscheidung |
| 0087 CSV-Fixtures konsolidieren | **0,5** | |
| 0086 toml-Semver-Warnung | **~0,2** | Versions-Pin |
| 0002 Eigene-Konten-Config | **0** | Nur noch HITL: echte IBANs in die gitignorierte Datei |
| 0008 Persistente Pseudonym-Zuordnung | **1** | Hängt an ADR-0010 |
| 0013 Iteration | **0** | Menschlicher Review-Loop, keine Agentenarbeit |

**Summe: rund 11–14 Sitzungen.** Am bisherigen Durchsatz gemessen eher zwei bis drei Wochen
als ein weiterer Monat.

## Empfohlene Reihenfolge

**Kurzer Weg zum ersten echten Ergebnis: `0004 → 0007 → 0012`, 3–4 Sitzungen.**
Danach sagt das Programm zum ersten Mal, wo das Geld hingeht. 0005 und 0006 verfeinern nur.

Begründung der Reihenfolge:

1. **0004 zuerst** — es ist das einzige Analyse-Ticket ohne offene Entscheidung, und es hebt die
   Cashflow-Zahlen von „berechnet" auf „stimmt". Ohne Netting zählt eine Überweisung von Giro auf
   Spar als Ausgabe.
2. **0007 danach** — die eigentliche Lieferung (Kostentreiber + Sparpotenzial). Vor 0004 sinnlos.
3. **0012** — bis dahin sind Ergebnisse nur per `sqlite3` von Hand sichtbar.
4. **0005/0006** verfeinern; 0005 gibt den heute weggeworfenen Vorkommen endlich einen Abnehmer.
5. **0084/0085** sind Betriebsthemen — nötig, bevor echte Daten dauerhaft in einer Datei leben.

## Vier Entscheidungen, die Agentenarbeit blockieren

Diese Wartezeiten stecken in **keiner** der Zahlen oben:

| Ticket | Offene Frage |
|---|---|
| 0010 | Was genau wird aus dem Freitext gescrubbt? (Heute nur getrimmt) |
| 0085 | Was passiert nach N Versuchen — Dead-Letter-Tabelle, Status-Spalte, Abbruch? |
| 0007 | Was zählt als „realistischer Spar-Kandidat"? |
| 0012 | Was soll der Bericht überhaupt aussagen? |

## Wo ich der Schätzung misstraue

- **Die Ticketlänge lügt.** Alle liegen bei 41–62 Zeilen, aber 0006 („ist das eine Gebühr?") ist
  mechanisch, während 0007 („was ist ein Spar-Kandidat?") eine Modellierungsfrage im selben
  Format ist. Eine kurze Spec für eine Urteilsfrage heißt meist: das Schwierige steht noch nicht
  drin. Bei **0005, 0007 und 0012** kann sich die Schätzung verdoppeln.
- **Agentenzeit ≠ Kalenderzeit** — siehe die vier Entscheidungen oben.
- **Teure Qualitätsschranken.** Sechs Gates, striktes `clippy`-„Don't Panic"-Set, Architektur-Test,
  Review-Kultur. Am 31.07. kostete allein eine `panic_in_result_fn`-Runde eine Iteration bei
  255 geänderten Zeilen. Eingepreist, aber der Grund gegen Stundenangaben.

## Zwei lose Enden ohne Ticket

- **Die GitHub-Seite ist leer.** 15 offene lokale Tickets, **null** offene GitHub-Issues (32
  insgesamt, alle closed). Damit ist [.rules/common/tickets.md](../../.rules/common/tickets.md)
  („Tickets doppelt anlegen — GitHub **und** `docs/issues/`") für jedes einzelne verletzt.
  Praktisch ist der Tracker derzeit [PROGRESS.md](../issues/PROGRESS.md). Nebenwirkung: die
  GitHub-Nummern sind PR-Nummern, weshalb das `Closes #83` im 0083-Commit ins Leere lief.
  Nachzuziehen wäre auch, wie lokale und GitHub-Nummer künftig verknüpft werden.
- **Die Branch-Strategie steht nur in einer Commit-Message.** `main` = Arbeitsstand (mit den
  eingecheckten Projekt-Skills), `dev` = Zuhause-Stand (die 24 namensgleichen Projekt-Skills
  entfernt, damit die neueren globalen aus `~/.agents` gelten). Nachlesbar in `1ecae24`, in
  keiner Doku. `dev` ist bislang **nicht** gepusht.
