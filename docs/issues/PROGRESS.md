# Issue-Fortschritt

Stand: 2026-07-16

## Neu (offen) — event-getriebene Pipeline (ADR-0008)

Aus der Fortschreibung der PRD nach [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md)
(event-getriebene Pipeline aus Slices mit Outbox) abgeleitet — deckt die bislang
unbelegten Messaging-/Pipeline-Teile ab. Die Fachfunktions-Issues (0004–0013)
bleiben gültig; nur ihre Verortung wandert in die Pipeline-Slices.

- [ ] **[0015-einlesen-slice-als-pipeline-consumer.md](0015-einlesen-slice-als-pipeline-consumer.md)**
  `einlesen`-Slice als Pipeline-Consumer (Umbau von `csv_camt_v8`), Message `Eingelesen`.
- [ ] **[0016-obfuskieren-slice-als-pipeline-consumer.md](0016-obfuskieren-slice-als-pipeline-consumer.md)**
  `obfuskieren`-Slice als Consumer von `Eingelesen`, Message `Obfuskiert` (Datenschutz-Grenze).

Vertagt (Zielbild ADR-0008, bewusst noch kein Issue): `report`-Consumer und der echte
`io`-Outbox-Broker (SQLite). Der `auswerten`-Consumer ist seit
[ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md) durch 0040–0045
belegt.

## Neu (offen) — Bau nach ADR-0013 (`fenstern` + `auswerten`-Split)

Zerlegung des Baus hinter dem akzeptierten
[ADR-0013](../adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md) in vertikale
Tracer-Bullet-Slices — jeder Schnitt geht durch alle Schichten und ist für sich verifizierbar
(gegen Fakes). **Out of scope bleibt** echtes `io` (SQLite/`sqlx`, Tag-Stamp-Persistenz,
Broker-Trigger) und der `report`. Die Fach-Analyzer sind **nicht** dupliziert — 0004/0005/0006/0007
bleiben gültig und wurden nur neu verdrahtet.

Prefactor:

- [x] **[0035-architektur-doku-auf-realen-stand-ziehen.md](0035-architektur-doku-auf-realen-stand-ziehen.md)**
  (2026-07-16, PR #33) Doku auf den realen Stand: kein `shared_kernel` mehr als existierende
  Crate, `fenstern` in Pipeline + Baureihenfolge, ADR-0013 als akzeptiert, Status auf
  `fenstern`/`auswerten`-Split. **Darüber hinaus** (siehe „Nachtrag" im Ticket): README auf ein
  drift-sicheres Minimum gekürzt und tote `shared_kernel`-Verweise aus Doc-Comments entfernt.
  → **0036 ist damit frei.**

`fenstern`-Slice:

- [ ] **[0036-fenstern-triade-ein-auswertungsfenster-materialisieren.md](0036-fenstern-triade-ein-auswertungsfenster-materialisieren.md)**
  Triade + Kern-Pfad: `AdvanceWindow` → `Auswertungsfenster` materialisieren → `FensterGeschlossen`.
  *Blockiert durch 0035.*
- [ ] **[0037-fenstern-selbst-fortsetzung-mehr-seiten-paging.md](0037-fenstern-selbst-fortsetzung-mehr-seiten-paging.md)**
  Selbst-Fortsetzung über `max(booking_day < from)`; Lücken übersprungen, Fenster nie leer.
  *Blockiert durch 0036.*
- [ ] **[0038-fenstern-startanalysis-kickoff-und-leere-datenbank.md](0038-fenstern-startanalysis-kickoff-und-leere-datenbank.md)**
  `StartAnalysis`-Anstoß über den `WindowAnchor`-Port; leerer Bestand → `Ok`, publiziert nichts.
  *Blockiert durch 0036.*
- [ ] **[0039-io-fenstern-end-to-end-ueber-generische-outboxconsumer.md](0039-io-fenstern-end-to-end-ueber-generische-outboxconsumer.md)**
  `fenstern` in `io` verdrahtet, end-to-end gegen Fakes. *Blockiert durch 0037, 0038.*

`auswerten`-Slice:

- [ ] **[0040-auswerten-triade-skelett-pro-fenster-reichweite.md](0040-auswerten-triade-skelett-pro-fenster-reichweite.md)**
  Triade + erster Handler (`FensterGeschlossen` → Aggregat per ID → Monats-Cashflow).
  *Blockiert durch 0036.*
- [ ] **[0041-fingerabdruck-als-domaenenfunktion.md](0041-fingerabdruck-als-domaenenfunktion.md)**
  `Fingerabdruck` als reine Domänenfunktion (inkl. Intermediär + Händler-Token).
  *Blockiert durch 0040.*
- [ ] **[0042-eigene-konten-config-und-gegenkonto-auf-token.md](0042-eigene-konten-config-und-gegenkonto-auf-token.md)**
  `Gegenkonto::Eigen|Fremd` auf Token + token-orientierte Eigene-Konten-Config. *Blockiert durch 0040.*
- [ ] **[0043-historie-vollstaendig-handler-und-fenster-uebergreifende-query.md](0043-historie-vollstaendig-handler-und-fenster-uebergreifende-query.md)**
  Zweiter Handler: `HistorieVollständig` → Read-Port-Query über alle Fenster des Laufs (kein Log).
  *Blockiert durch 0037, 0040.*
- [ ] **[0044-kategorisierer-port-fuer-unbekannte-fingerabdruecke.md](0044-kategorisierer-port-fuer-unbekannte-fingerabdruecke.md)**
  `Kategorisierer`-Port nur für unbekannte Fingerabdrücke, Egress-DTO ohne `Serialize`.
  *Blockiert durch 0041.*
- [ ] **[0045-io-auswerten-end-to-end.md](0045-io-auswerten-end-to-end.md)**
  `auswerten` in `io` verdrahtet; volle Pipeline gegen Fakes. *Blockiert durch 0039, 0043.*

Neu verdrahtete Fach-Analyzer (bestehende Issues): 0004 ← 0042 · 0006 ← 0040, 0004 ·
0005 ← 0041, 0043, 0004 · 0007 ← 0040, 0004, 0005, 0006.

Abschluss:

- [ ] **[0046-doku-drift-nach-den-merges-0036-0045-auf-den-realen-stand-ziehen.md](0046-doku-drift-nach-den-merges-0036-0045-auf-den-realen-stand-ziehen.md)**
  Doku **einmal am Ende** auf den realen Stand: Issue-Status, `PROGRESS.md`, Crate-Liste und
  Status-Abschnitt in CLAUDE.md, ADR-0013-Kopf. Gegenstück zum Prefactor 0035.
  *Blockiert durch 0045.*

## Erledigt

- [x] **[0032-shared-kernel-io-privat-machen-und-crate-in-io-aufl-sen-adr-0011.md](0032-shared-kernel-io-privat-machen-und-crate-in-io-aufl-sen-adr-0011.md)** (2026-07-15, PR #25)
  `shared_kernel` als eigene Crate aufgelöst und io-privat als Outbox-Modul in `io`
  untergebracht (ADR-0011). Squash-gemergt in `main`.
- [x] **[0033-einlesen-slice-oeffentliche-sichtbarkeit-auf-externe-flaeche-verengen.md](0033-einlesen-slice-oeffentliche-sichtbarkeit-auf-externe-flaeche-verengen.md)** (2026-07-15, PR #24)
  `einlesen`-Slice: öffentliche Sichtbarkeit auf die externe Fläche verengt. Squash-gemergt in `main`.
- [x] **[0034-obfuskierung-slice-oeffentliche-sichtbarkeit-auf-externe-flaeche-verengen.md](0034-obfuskierung-slice-oeffentliche-sichtbarkeit-auf-externe-flaeche-verengen.md)** (2026-07-15, PR #26)
  `obfuskierung`-Slice: öffentliche Sichtbarkeit auf die externe Fläche verengt, Roh-VOs
  über `from_parts` geschlossen. Squash-gemergt in `main`.
- [x] **[0023-error-ketten-vervollstaendigen-source-an-wrapper-errors.md](0023-error-ketten-vervollstaendigen-source-an-wrapper-errors.md)** (2026-07-15, PR #19)
  Error-Ketten vervollständigt: `ReadError::source()` reicht die innere Ursache je Variante
  weiter, `From<SourceError>`/`From<LineError>` lassen den Handler `?` nutzen; `HandleError`
  als Blatt-Error bestätigt (leeres `impl` korrekt). Redaktions-Invariante gewahrt.
- [x] **[0024-dto-domaene-mapping-auf-from-tryfrom-umstellen-tryfrom-wenn-fallibel-sonst-from.md](0024-dto-domaene-mapping-auf-from-tryfrom-umstellen-tryfrom-wenn-fallibel-sonst-from.md)** (2026-07-15, PR #19)
  Ein-eingabige DTO↔Domäne-Mappings auf die Standard-Konversions-Traits gehoben
  (fallibel → `TryFrom`, sonst `From`): `RawLine::try_from(&RawRecord)`, Summentyp-/
  Projektions-Mappings, obfuskierung-Egress-DTOs, `TransactionError → OutboxError`.
  `IngestedRow::to_booking` (mehr-eingabig) bleibt benannte Methode.
- [x] **[0025-vermeidbare-clones-auf-fehlerpfaden-entfernen.md](0025-vermeidbare-clones-auf-fehlerpfaden-entfernen.md)** (2026-07-15, PR #19)
  Vermeidbare Clones auf Fehlerpfaden entfernt: `SourceBridge` nimmt die `BookingId`
  borrowend (klont nur im `Ok`-Zweig), `ObfuscateBooking::into_id` eliminiert den Klon
  im Absent-Arm. Verhalten unverändert.
- [x] **[0026-api-guideline-derives-must-use-namen-und-kosmetik.md](0026-api-guideline-derives-must-use-namen-und-kosmetik.md)** (2026-07-15, PR #19)
  API-Guideline-Cleanup: `Debug` an `BookingLookup`, `#[must_use]` an konsumierenden
  TestApi-Buildern und VO-Konstruktoren/Gettern, Parameter `wert → value`,
  `split_once`-Kombinator, Deref weg, leeres `pub mod` → `#[doc]`.
- [x] **[0027-spec-testcode-aufraeumen-let-else-vec-wiederholung-fixture-helfer.md](0027-spec-testcode-aufraeumen-let-else-vec-wiederholung-fixture-helfer.md)** (2026-07-15, PR #19)
  Spec-Testcode idiomatischer: Doppel-Match → `let … else { panic!() }`, `vec![x; n]`,
  `fold` über Namensvarianten, spec-lokaler Fixture-Helfer, `.expect(…)`-Angleichung.
  Black-Box über die Fluent-TestApi unverändert.
- [x] **[0014-shared-kernel-outbox-vertraege.md](0014-shared-kernel-outbox-vertraege.md)** (2026-07-03)
  `shared_kernel` um die Outbox-Verträge erweitert; in der Gestalt verfeinert durch
  [ADR-0009](../adr/0009-kernel-split-und-idiomatische-outbox-vertraege.md)
  (Result/`OutboxError`, `publish`, `const MESSAGE_TYPE`, `TypedOutboxHandler`) und in
  `shared_kernel` (io-facing) + `application_kernel` (slice-intern) geschnitten.
- [x] **[0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md](0001-domaenen-skelett-buchung-fingerabdruck-obfuskiertertext-kategorieanfrage.md)** (2026-07-01)
  Domänen-Skelett: Buchung, Fingerabdruck, ObfuskierterText, KategorieAnfrage
- [x] **[0003-csv-camt-v8-adapter-zu-buchung.md](0003-csv-camt-v8-adapter-zu-buchung.md)** (2026-07-01)
  CSV-CAMT-v8-Adapter → `Buchung` (Umsatzquelle-Port), obfuskiert beim Mappen;
  getestet gegen synthetische Fixtures nach echtem Header. Echtdaten-Verifikation
  vertagt (Realdatei via CLI-Argument, sobald das Programm vollständig ist).
