# Issue Progress

Stand: 2026-10-12

## Abschlussplan — Fortschritt (Auftrag „Schritte 0–5 hands-off")

Plan: [2026-10-12-1343-abschlussplan.md](../plans/2026-10-12-1343-abschlussplan.md). Eine neue
Sitzung setzt **hier** fort: erledigt = gemergt in `dev` + gepusht; offen = noch nicht
begonnen; in Arbeit = Worktree unter `.claude/worktrees/` existiert.

| Schritt | Ticket | GitHub | Status | Commit |
|---|---|---|---|---|
| 0 Aufräumen | — | — | erledigt | ee6697d |
| 1 Schema-Version | [0084](0084-schema-an-einem-ort-und-versionsstempel-als-migrationsachse.md) | #101 | erledigt | c836e74 |
| 2 Zeilen-Schlüssel + lauf-eigene Fenster | [0088](0088-zeilen-schluessel-lauf-eigene-fenster-und-vorgemerktes-ueberspringen.md) | #105 | erledigt | 51ea0c8 |
| 3a Wiederkehr | [0005](0005-wiederkehr-erkennung.md) | #94 | erledigt | 9ed77a9 |
| 3b Scrubbing | [0010](0010-scrubbing-regeln-fuer-obfuskiertertext.md) | #97 | erledigt | 1ef9ab8 |
| 4a Cashflow/Pareto/Spar | [0007](0007-aderlass-pareto-ranking-und-spar-kandidaten.md) | #95 | erledigt | db8aa36 |
| 4b Kategorie-Register | [0009](0009-speicher-sqlite-fuer-dtos-und-kategorie-register.md) | #96 | erledigt | 1b70038 |
| 4c Dead-Letter | [0085](0085-poison-message-versuchszaehlung-und-dead-letter-statt-endloser-neuzustellung.md) | #102 | erledigt | e2bd671 |
| 5a HTML-Bericht | [0012](0012-html-bericht.md) | #99 | erledigt | af52ca3 |
| 5b Chat-Export/-Import | [0011](0011-kategorisierer-chat-payload-adapter.md) | #98 | erledigt | 6dc0ff0 |
| E2E vier Konten + Leak-Test | `crates/io/src/run/e2e_tests.rs` | — | erledigt | 1f8e1d7 |
| Schritt-6-Checkliste | [2026-10-12-schritt-6-checkliste.md](../plans/2026-10-12-schritt-6-checkliste.md) | — | erledigt | (dieser Commit) |

Weitere GitHub-Spiegel (Schritt 0): 0002 → #93, 0013 → #100, 0086 → #103, 0087 → #104.

Schritt 0 im Detail: 0084 `blocked` → `open` (0083 ist gebaut); 0008 geschlossen (ADR-0010 +
0059); Blocker 0008 aus 0009 gestrichen; neues Ticket 0088 für Schritt 2; Doku-Widersprüche
korrigiert (Entdopplung zeigt auf 0088 statt 0084 in CLAUDE.md, `lauf`, `io::run`; `io`-Crate-Doku
behauptet keinen Dead-Letter mehr; PRD-Stellen nachgezogen; ADR-Index bis 0015; ADR-0015 kündigt
kein `sqlx` mehr an).


## Erledigt

- [x] **[0006-gebuehren-klassifikation.md](0006-gebuehren-klassifikation.md)** (2026-07-31)
  Was die Bank selbst einzieht, wird am Buchungstext erkannt und je Monat und belastetem Konto
  ausgewiesen — **herausgehoben, nicht herausgenommen**: anders als die Umbuchung bleibt die
  Gebühr im Cashflow, weil sie echtes verlorenes Geld ist. Damit stehen die beiden gegenläufigen
  Analyzer nebeneinander; die Marker-Liste ist bewusst konservativ und noch gegen Markus' echte
  Auszüge zu verifizieren (HITL). **Zweiter gebauter Analyzer** des `auswerten`-Slice.
- [x] **[0004-umbuchung-netting.md](0004-umbuchung-netting.md)** (2026-07-31)
  Umbuchungen zwischen eigenen Konten fallen aus dem Cashflow heraus und stehen daneben als
  eigene Geldfluss-Sicht (je Monat und Rolle des Gegenkontos) — genettet wird je Zeile auf
  `Gegenkonto::Eigen`, nicht je Paar. Am echten Lauf belegt: der Juni-Abfluss sinkt von
  173060 auf 143060, die fehlenden 30000 stehen als Umbuchung aufs Sparkonto daneben.
  **Erster gebauter Analyzer** des `auswerten`-Slice.
- [x] **Status-Backfill: achtzehn längst gemergte Issues auf `closed` gezogen** (2026-07-29)
  Das Frontmatter hing der Realität hinterher — die Umsetzung war in allen Fällen gemerged,
  nur der Status stand noch auf `open`/`blocked`/`done`. Nachgezogen (je per Commit/PR
  verifiziert): [0003](0003-csv-camt-v8-adapter-zu-buchung.md),
  [0014](0014-shared-kernel-outbox-vertraege.md),
  [0015](0015-einlesen-slice-als-pipeline-consumer.md),
  [0016](0016-obfuskieren-slice-als-pipeline-consumer.md),
  [0017](0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md),
  [0018](0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md),
  [0019](0019-einlesen-consume-bridge-an-obfuskieren-angleichen.md),
  [0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md),
  [0021](0021-obfuskieren-end-to-end-ber-die-generische-outboxconsumer-in-io.md),
  [0022](0022-architecture-regel-application-kernel-ist-interface-only.md),
  [0028](0028-obfuskierung-terminal-fehlbares-read-save-publish-und-id-handoff.md),
  [0029](0029-einlesen-terminal-fehlbares-store-publish.md),
  [0030](0030-adr-0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md),
  [0031](0031-crates-auf-einheitliches-ziel-ordner-layout-umstrukturieren.md),
  [0046](0046-doku-drift-nach-den-merges-0036-0045-auf-den-realen-stand-ziehen.md),
  [0055](0055-waehrungs-bewusster-betrags-decode-minor-unit-per-iso-4217-von-der-domaene-hoch.md),
  [0057](0057-fenstern-store-real-und-end-to-end.md),
  [0079](0079-leak-tests-auf-das-diagnose-praefix-festnageln.md). Die zuvor uneinheitlichen
  Status-Werte `done`/`closed` sind damit auf **`closed`** vereinheitlicht.
- [x] **[0082-gemintete-zeilen-id-als-mintedid-newtype-durch-den-slice-store-seam-fuehren.md](0082-gemintete-zeilen-id-als-mintedid-newtype-durch-den-slice-store-seam-fuehren.md)** (2026-07-29)
  Gemintete Zeilen-ID reist als `MintedId`-Newtype durch `StoreSpec::stage`: konstruierbar einzig im Mint-Zweig (privates Feld, kein Konstruktor), geschrieben über `ToSql` — eine Schein-ID ist ein Compile-Fehler; Seam-Vertrag Richtung Slice unverändert
- [x] **[0081-obfuskierung-domaenenfehler-in-obfuscationerror-umbenennen.md](0081-obfuskierung-domaenenfehler-in-obfuscationerror-umbenennen.md)** (2026-07-29)
  obfuskierung-Domänenfehler heißt auf der ganzen öffentlichen Adapter-Fläche `ObfuscationError`; Namensdrift zu den Schwester-Slices und zum Do-Beispiel der Testkultur-Regel aufgelöst, `einlesen`s gleichnamiges Enum unberührt
- [x] **[0080-claude-md-baustand-auf-adr-0015-und-die-realen-stores-nachziehen.md](0080-claude-md-baustand-auf-adr-0015-und-die-realen-stores-nachziehen.md)** (2026-07-28)
  CLAUDE.md-Baustand nachgezogen: fünfzehn ADRs inkl. 0015, alle vier Ergebnis-Stores und alle Lese-Quellen als real ausgewiesen, Status- und Build-order-Abschnitt in Deckung
- [x] **[0078-obfuskierung-lese-seam-fehlbar-machen.md](0078-obfuskierung-lese-seam-fehlbar-machen.md)** (2026-07-28)
  obfuskierung-Lese-Seam fehlbar gemacht: dreiwertige Form (Err = Beschaffung gescheitert, Ok(None) = nichts vorhanden), io verschluckt keinen Datenbank-Fehler mehr
- [x] **[0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md](0077-auswerten-migrieren-f-nf-tabellen-fall-und-totes-sqlite-backed-unit-entfernen.md)** (2026-07-28)
  auswerten migrieren (Fünf-Tabellen-Fall) und totes sqlite_backed_unit entfernen
- [x] **[0076-fenstern-migrieren-zwei-tabellen-fall.md](0076-fenstern-migrieren-zwei-tabellen-fall.md)** (2026-07-27)
  fenstern migrieren (Zwei-Tabellen-Fall)
- [x] **[0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md](0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md)** (2026-07-27)
  obfuskierung migrieren und StoreUnavailable vereinheitlichen
- [x] **[0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md](0074-storespec-abstraktion-einf-hren-und-einlesen-migrieren.md)** (2026-07-27)
  StoreSpec-Abstraktion einführen und einlesen migrieren
