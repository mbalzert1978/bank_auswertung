# Domänengrenze sind öffentliche DTOs (Anti-Corruption-Layer); Tests und Persistenz docken nur dort an

> **Status (seit [ADR-0007](0007-modularer-monolith-feature-slices.md)):
> verfeinert.** Das Prinzip „nach außen nur DTOs, Tests docken am DTO-Vertrag an"
> gilt unverändert — aber die Grenze ist jetzt **je Slice** der public
> `<slice>_adapter` (mit slice-eigenen DTOs und einer Fluent-TestApi), nicht eine
> **einzige** Domänengrenze. „Jeder Port sein eigener DTO" bleibt tragend.

Die Domäne hat einen Anti-Corruption-Layer als einzige öffentliche Grenze:
Ein- und Ausgabe laufen ausschließlich über öffentliche DTOs. Niemand außerhalb
sieht Domänen-Interna. So bleibt das Innere frei refaktorierbar, solange der
DTO-Vertrag hält.

## Konsequenzen

- **Jeder Port spricht seinen eigenen DTO**, nicht einen gemeinsamen Gott-DTO.
  Insbesondere spricht der `Kategorisierer`-Port eigene schmale DTOs, **nicht**
  die `Buchung`-DTO: Hinweg `KategorieAnfrage` (Fingerabdruck + `ObfuskierterText`)
  als typisierter Obfuskierungs-Ausgang ([ADR-0003](0003-rust-als-sprache.md)),
  Rückweg `KategorieAntwort` (Fingerabdruck → Kategorie). Die Antwort schlüsselt
  auf dem Fingerabdruck, nie auf dem Text — Re-Identifikation ist damit
  strukturell ausgeschlossen.
- **Persistenz speichert DTOs**, nicht Domänenobjekte (SQLite). Das Mapping
  DTO ↔ Domäne lebt im Anti-Corruption-Layer. Beim Laden rekonstituiert der ACL
  das Kategorie-Register samt gelernter Korrekturen aus den DTOs zurück — die
  Domäne bleibt Source of Truth, die DTOs sind nur ihre Serialisierung.
- **Tests prüfen die Domäne als Black-Box** über die DTO-Grenze — gegen den
  öffentlichen Vertrag, ohne Implementierungsdetails zu kennen.
