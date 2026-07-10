# Plan 0018-1 — einlesen beschafft Zeilen über Datenquellen-Port statt per Command

Umsetzung von [Issue 0018](../issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md).
Branch: `feat/0018-einlesen-datenquellen-port`.

## Ausgangslage (verifiziert im Code)

- **Der Command transportiert heute den Batch.** `einlesen_domain::ReadTransactions` trägt
  `lines: Vec<RawLine>` (`commands/read_transactions.rs`); der `ReadHandler` iteriert
  `command.lines`, deutet je Zeile ein `LineAggregate` und staged/publiziert dann
  (`handler/mod.rs`). Die Domäne verlangt vier Ports (`Calendar`, `Money`, `Store`,
  `EventSink`); die Moduldoku von `ports/mod.rs` sagt ausdrücklich: „Die Rohdaten dagegen
  trägt der Batch selbst als `RawLine`-Werte, kein Port" — genau das dreht dieses Issue um.
- **Das Wire-DTO trägt die Zeilen mit.** `ReadTransactionsMessage` (adapter,
  `commands/read_transactions_message.rs`, serde) = `{ source: String, lines: Vec<RawRecord> }`;
  `source` ist reiner Routing-/Partition-Kontext, nur `lines` fließt ins Kommando.
- **Die Präsenz-Prüfung sitzt in der Consume-Bridge.** `ConsumeBridge::dispatch`
  (`bridges/consume_bridge.rs`) mappt den ganzen Batch batch-atomar per
  `RawRecord::to_raw_line` und wrappt die zwei terminalen Quellen in
  `TerminalError::MissingField(MissingMandatoryField) | ::Line(LineError)` →
  `OutboxError::Warning { reason }` (redigiert). `MissingMandatoryField`
  (`dto/raw_record.rs`) trägt nur den statischen Feldnamen (privates Feld, Display
  `"mandatory field missing: {field}"`, **kein** Accessor).
- **`LineError` ist reiner Deutungs-Fehler** (`AmountInvalid`/`CurrencyUnknown`/
  `BookingDayInvalid`/`ValueDateInvalid`, `error.rs`) — Präsenz lebt bewusst nicht dort.
- **Seams/Fakes:** `CalendarSeam`/`MoneySeam`/`StoreSeam` in `abstractions/`; `StoreSeam`
  wird (anders als Calendar/Money) **nicht** aus `lib.rs` re-exportiert — crate-intern, bis
  io ihn erfüllt. Die `ReadTestApi` (`testapi/read_test_api.rs`) sammelt `with_line`-Records
  heute in die **Nachrichten-Nutzlast** (`run` serialisiert `{source, lines}`) und fährt die
  echte Bridge-Kette gegen `FakeCalendarSeam`/`FakeMoneySeam`/`FakeStoreSeam`/
  `FakeOutboxPublisher`.
- **Specs** (`einlesen_specs/tests/einlesen.rs`) arrangieren ausschließlich über
  `with_line`/`run`; der Präsenz-Spec assertiert, dass der Dead-Letter-Grund den Feldnamen
  (`"amount"`) enthält — der Meldungstext ist also Spec-Kontrakt.
- **Das Muster existiert bereits im Nachbar-Slice:** `obfuskierung_domain::RawBookingSource`
  ist ein von der Domäne verlangter Beschaffungs-Port mit redigiertem
  `Malformed { field: &'static str }`; die `SourceBridge` mappt den roh-typisierten
  `IngestedSourceSeam` (`Option`-Rückgabe = nicht vorhanden) darauf und verlegt die
  Präsenz-Prüfung (`IngestedRow::to_booking` + `field()`-Accessor) hinter den Seam. Der neue
  einlesen-Port spiegelt dieses Muster — mit einem Unterschied: dort ist „abwesend"
  transient (Retry), hier ist jeder Beschaffungs-Fehler laut Issue **terminal**.
- **Zeiger-Semantik (0017) ist fertig und bleibt unberührt:** `Store::store → IngestionId`,
  `EventSink::publish(&IngestionId)`, `IngestedMessage` = reiner ID-Zeiger im
  `partition_key`.

Zu bauen ist damit **nur die Beschaffungs-Seite des einlesen-Slice**: Command entschlacken,
Datenquellen-Port + Quell-Seam + Port-Bridge einziehen, Präsenz-Prüfung dorthin verlegen,
TestApi/Fakes umhängen. `obfuskierung`, beide Kernel und die ausgehende Nachricht bleiben
unverändert — der Vertrag zwingt zu keinem Eingriff in fremde Slices.

## Designentscheidung

**Die Domäne verlangt einen fehlbaren Datenquellen-Port; der Adapter bindet ihn über eine
Port-Bridge an den öffentlichen Quell-Seam, hinter dem auch die Präsenz-Prüfung liegt.**
(Issue-Wortlaut: „Domäne setzt konkret an … Adapter setzt auf die Domäne um.")

- **Neuer Domänen-Port** (Vorbild `RawBookingSource`):
  `einlesen_domain::RawLineSource` mit
  `fn fetch(&self, configuration: &RunConfiguration) -> Result<Vec<RawLine>, SourceError>`.
  Er liefert den Batch bereits domänen-typisiert (`RawLine` wird wiederverwendet, kein neuer
  VO) und ist fehlbar mit domänen-typisiertem, redigiertem Fehler.
- **Neuer Domänen-Fehler** `SourceError` (in `error.rs`, hand-geschriebenes
  `Display`/`Error` wie `LineError`): `Unavailable { reason: String }` (Beschaffung
  misslungen — trägt die vom Seam durchgereichte **io-Diagnose**, sodass der
  Dead-Letter-Grund benennt, was genau schieflief) und
  `MissingMandatoryField { field: &'static str }` (Präsenz — redigiert per Konstruktion,
  nur der statische Feldname; Vorbild `BookingLookup::Malformed`). Beide **terminal**.
  Der Spec-Kontrakt der Präsenz-Meldung ist nur: **der Dead-Letter-Grund enthält den
  statischen Feldnamen** (der Spec assertiert `contains("amount")`, keinen Satz). Den
  Meldungssatz besitzt allein die **Domänen-Display** dieser Variante — der Adapter
  spiegelt ihn nicht (kein wortgleiches String-Duplikat über zwei Crates).
- **Handler-Fehlersumme** `ReadError { Source(SourceError), Line(LineError) }` (Domäne):
  `ReadHandler::handle` gibt `Result<(), ReadError>`; `LineError` selbst bleibt unverändert.
  Der adapter-lokale `TerminalError` schrumpft zum Newtype über `ReadError` (er existiert
  nur noch für die Orphan-Regel des `From<…> for OutboxError`); das Mapping auf
  `OutboxError::Warning` mit redigiertem Grund bleibt wie heute.
- **Neuer Auslöser-Command:** `ReadTransactions { configuration: RunConfiguration }` —
  keine Zeilen mehr. `RunConfiguration` ist ein **vorerst leerer** Domänen-Marker; der
  Handler reicht ihn an `fetch` durch (die Konfiguration parametrisiert die Beschaffung —
  so bleibt das Feld benutzt und der Zweck im Typ ablesbar). Das Wire-DTO
  `ReadTransactionsMessage` verliert `lines` **und** `source`: `source` duplizierte nur
  den Envelope-`partition_key` (ADR-0008-Envelope-Konvention — Produzent wie Konsument
  verlassen sich allein auf den `partition_key`), und ein Payload-Feld, das der Handler
  nie braucht, trägt nichts. Die Payload wird **feldlos** (`{}`), das DTO bleibt als
  reiner Deserialisierungs-Zieltyp bestehen — bewusst nicht als leerer Inhalt: leeren
  Envelope-Inhalt mappt die `TypedOutboxHandler`-Verdrahtung auf `OutboxError::Critical`.
  Ein Wire-Konfigurationsfeld kommt erst, wenn die Konfiguration Inhalt hat.
- **Neuer Quell-Seam** (der in ADR-0008/CLAUDE.md dokumentierte „source seam"):
  `einlesen_adapter::abstractions::SourceSeam` mit
  `fn fetch(&self) -> Result<Vec<RawRecord>, SourceUnavailable>` — roh-typisiert,
  spricht extern das bestehende `RawRecord`-DTO. Ein Fehlschlag der Beschaffung ist
  **nie Abwesenheit**, sondern ein terminaler Fehler — darum ein `Result`, kein
  überladenes `Option::None` (die `Option`-Form des `IngestedSourceSeam` kodiert dort
  echtes „(noch) nicht da", einen transienten Normal-Ausgang; diesen Ausgang gibt es
  hier nicht). `SourceUnavailable { message: String }` ist crate-intern
  (`Display`/`Error` von Hand) und trägt die **io-Diagnose**: io speist den Text seines
  eigenen Fehler-Results ein (Pfad, OS-/Parse-Fehler), damit nachvollziehbar bleibt,
  was genau schieflief. **Redaktions-Pflicht am Seam dokumentiert** (bewusste
  Präzisierung des Issue-Worts „redigiert"): die Meldung beschreibt die
  Beschaffungs-Mechanik und zitiert **nie** Satz-/Feldinhalt — anders als beim
  statischen Feldnamen ist das Vertrag, nicht Konstruktion; der Text bleibt lokal
  (Dead-Letter), kein Egress. io erfüllt den Seam weiterhin domänen-blind;
  `Ok(vec![])` = leerer Batch (gültig, es passiert nichts). Wie `StoreSeam`
  **crate-intern** (kein `lib.rs`-Re-Export), bis io ihn erfüllt.
- **Neue Port-Bridge** `SourceBridge` (Vorbild obfuskierungs `SourceBridge`): erfüllt
  `RawLineSource`, delegiert an den Seam; `Err(SourceUnavailable { message })` →
  `SourceError::Unavailable { reason: message }` (die io-Diagnose wird durchgereicht);
  sonst
  batch-atomares `to_raw_line`-`collect` — das erste fehlende Pflichtfeld wird (wie ein
  io-Beschaffungsfehler) auf `SourceError::MissingMandatoryField` abgebildet. Damit
  **wandert die Präsenz-Prüfung aus der Consume-Bridge hinter den Seam**; sie läuft
  weiterhin vollständig, bevor der Handler eine einzige Zeile deutet (batch-atomar wie
  heute).
- **Unverändert:** Fan-out (eine `Ingested`-Nachricht je gültiger Zeile), Zeiger-Semantik
  (Tabelle = Wahrheit, Message = Pointer), `Calendar`/`Money`/`Store`/`EventSink` samt
  Bridges, `IngestedMessage`, Dead-Letter-Weg (`OutboxError::Warning`, io lettert
  batch-atomar), **keine** separate Fehler-Nachricht, keine neuen externen
  `[dependencies]`.
- **Neuer Spec „Quelle nicht beschaffbar":** die Abbildung Seam-Fehlschlag → Dead-Letter
  mit durchgereichter io-Diagnose ist beobachtbares Slice-Verhalten der Bridge (analog
  Präsenz), keine io-Format-Mechanik — sie wird darum spezifiziert (der Grund trägt die
  arrangierte Diagnose-Meldung: Nachvollziehbarkeit ist Verhalten, kein Detail); die
  reale Lese-Mechanik dahinter bleibt untestiert (io-Sache).

### Offene Detailfragen (explizit offen, im Plan nur mit Default belegt)

- **O1 — Inhalt der `RunConfiguration`:** was sie beim io-Schritt trägt (Quellpfad?
  Quell-/Datei-ID? Format-Wahl?) und ob die **Quelle aus dem Envelope-`partition_key`**
  dann in sie hineinfließt oder reiner Routing-Kontext bleibt. **Vertagt auf den io-Schritt**; bis dahin leerer
  Marker, der Seam bleibt arglos (die Bridge verwirft die leere Konfiguration).
- **O2 — Lesart „optionales Konfigurationsfeld":** der Plan wählt einen **non-`Option`
  leeren Marker** („nichts konfiguriert" liegt im Typ selbst; `Option<LeererMarker>`
  kodierte denselben Zustand doppelt). Offen, falls das Issue wörtlich
  `Option<RunConfiguration>` meint.
- **O3 — serde-Derives von `RawRecord`:** nach dem Umbau reist `RawRecord` nicht mehr über
  die Outbox — die Derives (`Serialize`/`Deserialize`) sind toter Ballast. Der Plan
  **entfernt sie** (Wiedereinführung ist additiv, falls der io-CSV-Parse später per
  `serde::Deserialize` mappen will). Offen bis zur io-Revision.
- ~~O4 — Fehlerform des Seams~~ **entschieden** (Review + Markus): ein fehlgeschlagener
  Bezug ist nie ein `None`, sondern terminal → `Result`; `SourceUnavailable` trägt die
  io-Diagnose als `message: String` (Redaktions-Pflicht am Seam dokumentiert, s.
  Designentscheidung).

## Aufgaben (in Abhängigkeitsreihenfolge)

### T1 — einlesen_domain

- `error.rs`: `SourceError` (`Unavailable { reason: String }`,
  `MissingMandatoryField { field: &'static str }`) und die Fehlersumme `ReadError`
  (`Source`/`Line`); Display von Hand — Präsenz redigiert (nur der statische Feldname;
  diese Domänen-Display ist der **einzige Eigentümer** des Meldungssatzes, Kontrakt ist
  allein „Grund enthält den Feldnamen"), `Unavailable` trägt die durchgereichte
  io-Diagnose; nie ein Satz-/Feldinhalt.
- `commands/read_transactions.rs`: `ReadTransactions` → Auslöser
  `{ configuration: RunConfiguration }`; leerer Marker `RunConfiguration` daneben; Doku
  (Command = Auslöser, nicht Transporter).
- `ports/raw_line_source.rs` (neu): `RawLineSource::fetch(&self, &RunConfiguration)
  -> Result<Vec<RawLine>, SourceError>`; Doku (fehlbar, terminal, redigiert; io erfüllt
  später über den Seam, Fake in den Specs).
- `ports/mod.rs`: Port einhängen; Moduldoku korrigieren („Rohdaten trägt der Batch, kein
  Port" gilt nicht mehr).
- `handler/mod.rs`: fünfter Port `&dyn RawLineSource`; Ablauf `fetch → je Zeile deuten →
  stagen + ID publizieren`; Rückgabe `Result<(), ReadError>`; Doku.
- `lib.rs`: `RawLineSource`, `SourceError`, `ReadError`, `RunConfiguration` exportieren;
  Datenschutz-Absatz nachziehen (rohe Werte reisen nicht mehr per Outbox-Nachricht,
  sondern in-process über den Quell-Seam); `commands/raw_line.rs`-Doku (Verweis auf den
  Command-Batch) anpassen.
- **Verifikation:** `cargo build -p einlesen_domain`.

### T2 — einlesen_adapter (Seam/Bridge/Wire)

- `abstractions/source_seam.rs` (neu): `SourceSeam::fetch(&self)
  -> Result<Vec<RawRecord>, SourceUnavailable>` mit
  `SourceUnavailable { message: String }` (`Display`/`Error` von Hand — io speist den
  Text seines Fehler-Results ein); crate-intern wie `StoreSeam` (kein
  `lib.rs`-Re-Export); Doku (io erfüllt; CSV-CAMT-v8 = eine Eingabeform dahinter;
  Fehlschlag ist terminal, nie Abwesenheit; Redaktions-Pflicht: Beschaffungs-Mechanik,
  nie Satz-/Feldinhalt).
- `abstractions/mod.rs`: einhängen.
- `dto/raw_record.rs`: `field()`-Accessor auf `MissingMandatoryField` (Vorbild
  obfuskierung); prüfen, ob dessen `Display`/`Error`-Impls nach dem Umbau noch Aufrufer
  haben — ohne Aufrufer **streichen statt spiegeln** (den Meldungssatz besitzt die
  Domänen-Display, s. Designentscheidung); serde-Derives entfernen (O3); Doku (Seam-DTO
  statt Wire-DTO).
- `bridges/source_bridge.rs` (neu): erfüllt `RawLineSource` über den Seam —
  `Err(SourceUnavailable { message })` → `SourceError::Unavailable { reason: message }`;
  sonst batch-atomares `to_raw_line`-`collect`, fehlendes
  Pflichtfeld → `SourceError::MissingMandatoryField { field }`; verwirft die (leere)
  `RunConfiguration` vorerst; Vorbild obfuskierungs `SourceBridge`.
- `commands/read_transactions_message.rs`: `lines` **und** `source` streichen — die
  Payload wird feldlos (`{}`), die Quelle lebt allein im Envelope-`partition_key`; das
  DTO bleibt reiner Deserialisierungs-Zieltyp; Doku.
- `bridges/consume_bridge.rs`: `dispatch` baut nur noch den Auslöser
  (`ReadTransactions::new(RunConfiguration)`) und dispatcht; `TerminalError` → Newtype über
  `ReadError` (die `MissingField`-Variante entfällt); `From<TerminalError> for OutboxError`
  unverändert redigiert.
- `bridges/mod.rs`, `commands/mod.rs`, `lib.rs`: Verdrahtung + Moduldoku (die Nachricht
  trägt keinen Batch mehr; der Batch kommt über den Quell-Seam).
- **Verifikation:** `cargo build -p einlesen_adapter`.

### T3 — einlesen_adapter (Fakes/TestApi)

- `testapi/fakes/fake_source_seam.rs` (neu): hält die arrangierten `RawRecord`s und eine
  optional arrangierte Ausfall-Diagnose; `fetch()` → `Err(SourceUnavailable { message })`
  mit der arrangierten Meldung, sonst `Ok(records)`.
- `testapi/fakes/mod.rs`: einhängen.
- `testapi/read_test_api.rs`: `with_line` speist den **Fake hinter dem Port** statt der
  Nachrichten-Nutzlast; neu `with_unavailable_source(message)`; `run` serialisiert die
  feldlose Message (`{}`) — die Quelle reist nur noch als `partition_key` des Envelopes —
  und verdrahtet die `SourceBridge` in die echte Kette; Beobachter (`published_*`,
  `stored_*`) unverändert; Doku.
- **Verifikation:** `cargo build -p einlesen_adapter --all-targets`.

### T4 — einlesen_specs

- Moduldoku: Ablauf nachziehen (Command = Auslöser; der Batch kommt über den Quell-Seam;
  die Präsenz-Prüfung liegt hinter dem Seam in der Source-Bridge — weiterhin batch-atomar,
  redigiert, Slice-Verhalten).
- Bestehende Szenarien (Happy-Path, Fan-out, Feld-Treue, Redaktion, fehlendes Pflichtfeld,
  optionaler Empfänger, unbekannte Währung): Arrangement und Assertions **unverändert**
  (`with_line`/`run` bleiben die Fläche); nur Kommentartexte anpassen, die den
  Command-Batch beschreiben.
- Neu: `eine_nicht_beschaffbare_quelle_lettert_ins_dead_letter` —
  `with_unavailable_source(meldung)` → `Err(OutboxError::Warning)`, dessen Grund die
  arrangierte io-Diagnose enthält (Nachvollziehbarkeit ist beobachtbares Verhalten);
  nichts gestaged, nichts publiziert.
- **Verifikation:** `cargo test -p einlesen_specs`.

### T5 — Gesamtverifikation

- `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`,
  `cargo test -p architecture` — alle grün.
- Belegen, dass an Code **nur** `einlesen_*`-Crates geändert sind (`git diff --stat`):
  `obfuskierung`, beide Kernel, `architecture` und der Hook bleiben unberührt.
- ADR-0008 nachziehen, damit es dem As-built nicht widerspricht: am Absatz „Der
  **Command trägt, was der Handler braucht** — … den ganzen Batch (alle Zeilen)"
  (§ „Der Handler arbeitet auf Domänen-Typen") einen Verfeinerungs-Blockquote nach dem
  bestehenden Muster des ADRs ergänzen — sinngemäß
  `> **Verfeinert durch [Issue 0018](…):** der Command ist reiner Auslöser; den Batch
  beschafft der Handler über den Datenquellen-Port.`
- `CLAUDE.md` §einlesen als As-built nachziehen („Consumes a `ReadTransactions` command
  (whole batch)" → Auslöser + Datenquellen-Port).

## Abnahmekriterien (aus dem Issue)

- [ ] Der `einlesen`-Command trägt keine Zeilen-Nutzlast mehr (Auslöser mit vorerst leerem
      Konfigurationsfeld); auch das Wire-DTO trägt keine Zeilen mehr (und kein
      `source`-Duplikat des Envelope-`partition_key` — die Payload ist feldlos). (T1/T2)
- [ ] Der Handler bezieht die Rohzeilen über den neuen, von der Domäne verlangten
      Datenquellen-Port (`RawLineSource`); die Verdrahtung an die Außenwelt besorgt die
      Adapter-Port-Bridge (`SourceBridge` → `SourceSeam`; Fake, später io). (T1/T2/T3)
- [ ] Der Port ist fehlbar und gibt bei misslungener Beschaffung einen domänen-typisierten,
      redigierten Fehler zurück (`SourceError`); die Präsenz-Prüfung liegt hinter dem Seam
      und wird auf diesen Fehler abgebildet. (T1/T2)
- [ ] Beschaffungs-/Präsenz- und Deutungs-Fehler bubblen terminal zu
      `OutboxError::Warning` (Dead-Letter batch-atomar); keine separate Fehler-Nachricht;
      die Redaktion trägt bis in die Dead-Letter-Meldung. (T2/T4)
- [ ] Fan-out (eine „Eingelesen"-Nachricht je gültiger Zeile) und Zeiger-Semantik bleiben
      unverändert. (T1/T4 — Assertions der bestehenden Specs unverändert grün)
- [ ] Die Slice-Specs bleiben Black-Box über die Fluent-TestApi; die Rohzeilen speisen den
      Fake hinter dem Port; die bestehenden Ablehnungs-Szenarien (fehlendes Pflichtfeld →
      batch-atomarer Dead-Letter; unbekannte Währung) bleiben grün. (T3/T4)
- [ ] `cargo build` / `cargo test` / `cargo clippy --all-targets` / `cargo fmt --check`
      und `cargo test -p architecture` sind grün. (T5)
