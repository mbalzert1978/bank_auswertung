# Issue Progress

Stand: 2026-07-29

## Erledigt

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
