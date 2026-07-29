# Issue Progress

Stand: 2026-07-29

## Erledigt

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
