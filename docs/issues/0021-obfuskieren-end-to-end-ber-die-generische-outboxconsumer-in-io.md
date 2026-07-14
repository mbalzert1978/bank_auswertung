---
id: "0021"
title: obfuskieren end-to-end über die generische OutboxConsumer in io
status: open
---

# obfuskieren end-to-end über die generische OutboxConsumer in io

## Parent

Umsetzung von [ADR-0011](../adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md)
(„Der Vertical Slice ist eine reine Pipeline — die Outbox-Mechanik lebt in `io`"),
Wayfinder-Planungskarte
[#8](https://github.com/mbalzert1978/bank_auswertung/issues/8). **Zweite Scheibe** nach
dem `einlesen`-Slice ([#13](https://github.com/mbalzert1978/bank_auswertung/issues/13),
lokal [0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)),
aufsetzend auf der dort gebauten Consumer-Maschinerie.

## What to build

Der `obfuskierung`-Slice läuft **end-to-end über die generische `OutboxConsumer<T:
TransactionalHandler>` in `io`** statt über die slice-eigene `ConsumeBridge`. Ein `Ingested`-Envelope
(Aggregat-Zeiger im `partition_key`) geht durch `io` (Envelope extrahieren →
slice-gelieferter `CommandDecoder` mit nackten Strings → `ObfuscateBooking { id }` →
`handle`), staged das obfuskierte Aggregat und publiziert die `Obfuscated`-Nachricht; ein
**terminaler Fehler** wird über das `Display` des nackten Domänen-Fehlers **redigiert
dead-lettered**. Danach ist der `obfuskierung_adapter` **restlos outbox-frei** und bindet
nur noch `application_kernel`.

(Sprach-Konvention: die Nachricht heißt im Code `Obfuscated` — `MESSAGE_TYPE = "Obfuscated"`,
`ObfuscatedMessage`; „obfuskiert" bleibt der deutsche Prosa-Begriff, kein Rename.)

Diese Scheibe **wiederverwendet** die in [0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)
gebaute `io`-Maschinerie unverändert (generische `OutboxConsumer<T:
TransactionalHandler>`, `application_kernel::Publisher` →
`shared_kernel::OutboxPublisher`, `CommandDecoder`-Vertrag Shape Y,
`message_type`-Auflösung) und migriert `obfuskierung` als **zweiten** Slice darauf. Wie
`io::einlesen` erhält `obfuskierung` dafür in `io` einen slice-spezifischen
`TransactionalHandler` samt `UnitFactory`, **Zeile für Zeile parallel** zu einlesen: die
geöffnete Unit erfüllt genau die **committenden** Ports `ObfuscatedStoreSeam`,
`OutboxPublisher` und `UnitOfWork` (drei Traits, wie einlesens Unit `StoreSeam` +
`OutboxPublisher` + `UnitOfWork`). Das **Lesen gehört nicht in die Transaktion**:
`IngestedSourceSeam` und der reine `ObfuscatorSeam` sind **message-unabhängige Seams**, die
der Aufrufer separat hereinreicht — die direkten Gegenparts zu einlesens `source` bzw.
`calendar`/`money`. Der TransactionalHandler baut den Slice-Handler aus diesen Seams **plus**
der Unit und committet nach einem erfolgreichen Handler-Lauf genau einmal — so committen die
beiden staged **Schreibungen** (Store + Publish) gemeinsam oder gar nicht, während der
idempotente Lesevorgang (die einlesen-Zeile ist append-only/unveränderlich) außerhalb bleibt
und die Transaktion nicht weitet.

Mit `obfuskierung` outbox-frei bindet **kein bestehender Slice-Adapter** mehr direkt
`shared_kernel`; die Enforcement-Regel „`_adapter` bindet kein `shared_kernel`" (in
[0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)
bewusst zurückgestellt) kann jetzt greifen. Die workspace-weite Nachziehung anderer Crates
bleibt den dafür vorgesehenen Follow-up-Issues vorbehalten; diese Scheibe zieht nur
`obfuskierung_adapter`, `obfuskierung_specs`, seine `io`-Verdrahtung und die
Adapter-Kantenregel nach.

Die **Fehler-Taktik wird uniform terminal**: der bisherige
`HandleError::NotYetAvailable`-→-Retry-Zweig für einen absenten Datensatz entfällt. Das
trägt **nicht** die `obfuskierung`-`UnitOfWork` (die regelt nur die eigene Atomarität
Lesen-aus-einlesen + Schreiben-nach-obfuskiert und sagt **nichts** darüber, ob die
einlesen-Zeile existiert), sondern das **Zusammenspiel zweier Garantien**: (1) `einlesen`
committet Roh-Zeile **und** `Ingested`-Nachricht **atomar in einer** `UnitOfWork`
([0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)),
und (2) der Broker stellt eine Nachricht erst **nach** dem Commit ihrer erzeugenden
Transaktion zu (read-after-commit-Sichtbarkeit, ADR-0011 / Planungskarte „Die Grenze Slice
⟷ Outbox definieren", [#9](https://github.com/mbalzert1978/bank_auswertung/issues/9)). Wenn
`obfuskierung` die `Ingested`-Nachricht sieht, **liegt** die Roh-Zeile also im
einlesen-Speicher; eine absente Zeile ist damit ein **gebrochenes Invariant** (korrupter
Speicher / falsche ID) — terminal dead-lettered statt retried.

Die **Ausgänge bleiben im Typ, nur der Fork verschwindet**: `RawBookingSource` muss eine
fehlende Zeile weiterhin **melden** können, also bleiben `BookingLookup::Absent` und
`HandleError::NotYetAvailable` als **Ergebnisse** erhalten. Gelöscht wird allein die
**Abbildung** — die bisherige `ConsumeBridge`-Verzweigung `NotYetAvailable → Retry` /
`Malformed → Warning` entfällt, weil `io` **jeden** `HandleError` uniform über
`TransactionError::Handler → OutboxError::Warning` terminal spiegelt (kein slice-eigener
Fork mehr). `NotYetAvailable`s `Display` trägt die **opake** `BookingId` (`"…found in
source: {id}"`); die ID ist kein sensibler Wert, aber sie wird jetzt als **persistierter
Dead-Letter-Grund** gespiegelt — das ist mit der Redaktions-Regel vereinbar (kein Rohwert),
muss aber bewusst so benannt sein. `MalformedSource` bleibt wie bisher **redigiert terminal**
(nur der statische Feldname, nie ein Rohwert). Der `assert_not_impl_any!`-Guard
„kein `Serialize`" auf `ObfuscatedBooking` bleibt **unverändert** — er ist orthogonal zur
Outbox-Mechanik und sichert die Datenschutz-Grenze
([ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md),
[ADR-0010](../adr/0010-seed-basierte-pseudonym-stabilitaet.md)).

## Acceptance criteria

- [ ] `obfuskierung_adapter`: eigener `CommandDecoder` (`partition_key` →
  `ObfuscateBooking { id }`, Shape Y — kein Outbox-Typ am Eingang; ein **leerer/blanker**
  `partition_key` liefert `None` → `Critical` auf Envelope-Ebene, wie io es für ungültigen
  Inhalt tut — nicht als Handler-Fehler durchreichen), Publisher-Bridge auf
  `application_kernel::Publisher` umgehängt, `ConsumeBridge` + jede `shared_kernel`-Nutzung
  entfernt → bindet nur noch `application_kernel`.
- [ ] Fehler-Taktik uniform terminal: der `NotYetAvailable`-→-Retry-Zweig **in der
  Abbildung** entfällt (die `ConsumeBridge`-Verzweigung verschwindet ganz; `io` spiegelt
  jeden `HandleError` über `TransactionError::Handler → Warning`), während
  `BookingLookup::Absent`/`HandleError::NotYetAvailable` als **Ergebnis-Varianten** bleiben
  (die Quelle muss eine fehlende Zeile melden können). Ein absenter Datensatz wird terminal
  dead-lettered — gebrochenes Invariant, weil `einlesen` Zeile+Nachricht atomar committet und
  der Broker erst danach zustellt (nicht: die `obfuskierung`-`UnitOfWork` schließe „absent"
  aus). `MalformedSource` bleibt redigiert terminal (nur Feldname, nie ein Rohwert);
  `NotYetAvailable` spiegelt nur die opake `BookingId`.
- [ ] Die Dokumentation an `BookingLookup::Absent`, `HandleError::NotYetAvailable`,
  `RawBookingSource`/`SourceBridge` und dem Handler beschreibt „absent" nicht mehr als
  transienten Retry-Fall, sondern als terminales gebrochenes Invariant; die Ergebnis-Varianten
  selbst bleiben unverändert erhalten.
- [ ] `io` erhält analog zu `io::einlesen` einen
  `ObfuskierungTransactionalHandler` samt `UnitFactory`. Die Unit erfüllt **nur die
  committenden** Ports `ObfuscatedStoreSeam`, `OutboxPublisher` und `UnitOfWork` (drei
  Traits, Spiegel von einlesens Unit); `IngestedSourceSeam` und `ObfuscatorSeam` sind
  **message-unabhängige Seams**, die der Aufrufer separat hereinreicht (wie einlesens
  `source`/`calendar`/`money`) — der Lesevorgang bleibt **außerhalb** der Transaktion. Der
  TransactionalHandler baut Decoder und Slice-Handler aus dieser Verdrahtung, mappt den
  nackten `HandleError` auf `TransactionError::Handler` und committet nach Erfolg genau
  einmal. Die (in
  [0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)
  gebaute) `OutboxConsumer<T: TransactionalHandler>` und ihre Fehlerabbildung bleiben
  unverändert; der neue Consumer wird per `message_type` aufgelöst.
- [ ] `obfuskierung_specs`/TestApi auf **Handler-Ebene** über den
  `application_kernel`-Vertrag (Command/Strings rein → staged obfuskiertes Aggregat +
  publizierte `Obfuscated`-Nachricht bzw. Domänen-Fehler); keine `OutboxMessage`/`OutboxError`-Asserts
  mehr. Der `assert_not_impl_any!`-„kein `Serialize`"-Guard auf `ObfuscatedBooking` bleibt
  unverändert.
- [ ] `io`-Integrationstest: `Ingested`-Envelope → `OutboxConsumer` → echtes `obfuskierung`
  → atomar committetes obfuskiertes Aggregat + publizierte `Obfuscated`-Nachricht. Bei
  Handler-Fehler werden weder Store noch Publish committet und der Fehler wird redigiert
  dead-lettered; ein Commit-Fehler wird als einziger Retry-Fall gespiegelt.
- [ ] Mit `obfuskierung` migriert bindet **kein bestehender Slice-Adapter mehr
  `shared_kernel`**: die
  Enforcement-Regel „`_adapter` bindet kein `shared_kernel`" (in
  [0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)
  zurückgestellt) wird in `architecture` ergänzt. Eine stärkere workspace-weite Regel ist
  nicht Teil dieser Scheibe; weitere Crates werden in ihren Follow-up-Issues nachgezogen.
- [ ] Volle Suite grün: `cargo build`/`test`/`clippy --all-targets`/`fmt --check` +
  `cargo test -p architecture`.

## Blocked by

None - can start immediately. [0020](0020-einlesen-end-to-end-ber-die-generische-outboxconsumer-in-io-kernel-flip.md)
hat die generische `OutboxConsumer<T: TransactionalHandler>`-Maschinerie in `io`, den
`CommandDecoder`-Vertrag (Shape Y) in `application_kernel` und den Kernel-Flip bereits
gebaut; diese Scheibe migriert `obfuskierung` als zweiten Slice darauf.
