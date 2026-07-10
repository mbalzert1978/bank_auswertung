# Event-getriebene Pipeline aus Slices mit Outbox — jeder Schritt ein Consumer

Die Auswertung ist eine **Pipeline**; jeder Schritt ist ein **eigener Slice**, per
**Outbox** vom vorigen getriggert. Dieses ADR macht das zur tragenden Architektur und
**verfeinert/supersedet** dazu Aussagen von [ADR-0001](0001-hexagonale-architektur.md),
[0003](0003-rust-als-sprache.md), [0004](0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)
und [0007](0007-modularer-monolith-feature-slices.md) (s. „Was dieses ADR supersedet").
Vorbild: die interne C#-Referenz `examples/` (`Downloader`/`Stager`, `Outbox`, `Common`).

## Die Pipeline

```
einlesen → obfuskieren → auswerten → report
                             (auswerten ruft das LLM-Orakel bei Bedarf selbst, kein eigener Schritt)
```

Jeder Schritt ist eine Slice-Triade (`_domain`/`_adapter`/`_specs`, ADR-0007) und ein
**Consumer**: er verarbeitet **ein** Command und schreibt in **einer** Transaktion (a) in
seine **eigene Tabelle** und (b) eine **Outbox-Message** für den nächsten Schritt
(Outbox-Pattern → atomar, at-least-once, wiederaufsetzbar). Kein Schritt ruft den nächsten
synchron; die Kopplung ist allein die Message.

- **einlesen** — der Kopf. Sein Handler spricht ein **Repository** an, das die Rohdaten
  liefert, **prüft/validiert** sie und erfasst sie. Er heißt nach dieser Verantwortung,
  **nicht** nach dem Format: **CSV-CAMT-v8 ist nur eine Eingabeform** hinter dem Quell-Seam
  (XML-camt/MT940 später = weiterer IO-Adapter, kein neuer Slice). Schreibt Roh-Tabelle +
  Message `Eingelesen`.
- **obfuskieren** — konsumiert `Eingelesen`, vergibt `KONTO_x`/`PERSON_x` **seed-basiert**
  ([ADR-0010](0010-seed-basierte-pseudonym-stabilitaet.md)), scrubt Freitext.
  **Datenschutz-Grenze** (s. u.). Schreibt `obfuskiert`-Tabelle + Message `Obfuskiert`.
- **auswerten** — konsumiert `Obfuskiert`, macht die Analyse (Recurrence, Gebühren,
  Cashflow, Pareto) + Kategorie-Register ([ADR-0004](0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md)).
  **Kategorisieren ist kein eigener Schritt:** hält die Domäne einen Fingerabdruck für
  unbekannt, ruft der Handler den injizierten `Kategorisierer`-Port (LLM als Orakel,
  Export→Chat→Import) — eine **Domänen-Entscheidung** „brauche ich das Orakel?", kein
  Pipeline-Schritt. Bewusste **Asymmetrie** zu `obfuskieren`: beide bedeuten einen externen
  Call, aber `obfuskieren` ist eine **mechanische, immer nötige Grenze** (eigener Schritt),
  Kategorisieren eine **fachliche Bedarfs-Entscheidung** der Domäne — darum eingefaltet.
  Schreibt Ergebnis + Message → report.
- **report** — konsumiert das Endergebnis → HTML.

## Der Handler arbeitet auf Domänen-Typen

Ein Slice hat **einen** Handler (`shared_kernel::Handler` mit `Command`/`Response`/`Error`)
— **interne Domänen-Typen**, kein public IO kennt sie. Er bekommt seine **Domänen-Ports**
(in `<slice>_domain` definiert) und einen `Publisher` **injiziert** und **orchestriert das
Aggregat über dessen Methoden** (reiche Domäne). Er kennt **keinen** konkreten Adapter,
**keinen** fremden Slice-Typ, **keine** Wire-/IO-Form. Vorbild:
`Stager/Handlers/StagerHandler.cs`. `Response`/`Error` tragen bewusst **keine**
Trait-Schranke: das Mappen `Error` → `HandleOutcome` (`Handled`/`Retry`/`Faulted`) ist
adapterspezifisch und lebt in der Consume-Bridge, nicht im Kernel-Vertrag.

> **Verfeinert durch [ADR-0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md):**
> `HandleOutcome`/`Severity` → `OutboxError`; das Mapping ist `Error → OutboxError` über
> `Into<OutboxError>`, `handle` gibt `Result<(), OutboxError>`. Dass `Response`/`Error` keine
> Schranke tragen, bleibt.

Der **Command trägt, was der Handler braucht** — für den Ein-Rutsch-Betrieb den ganzen Batch
(alle Zeilen), nicht zeilenweise.

## Der Adapter ist die Anpassung an die Außenwelt

- **Consume-Bridge** (`OutboxHandlerBase`-Muster): implementiert den public
  `OutboxHandler`-Seam — deserialisiert `OutboxMessage.content` → Domänen-`Command`,
  dispatcht an den Handler, mappt `Response`/`Error` → `HandleOutcome`. Erst **hier** beginnt
  die Außenwelt.
- **Publisher-Bridge**: implementiert den Domänen-`Publisher`-Port — mappt den
  Domänen-Command auf sein **Wire-DTO** (`OutboxCommand`) und legt es über den
  `OutboxPublisher` in die Outbox.
- **Port-Bridges**: implementieren die Domänen-Ports durch Delegation an public Seams —
  eigene IO-Seams **oder** die public `Abstractions` eines Nachbar-Slice (Vorbild
  `Stager/Adapter/DownloadProbeAdapter.cs`). Der Handler kennt nur den eigenen Domänen-Port;
  das Mappen der fremden Sprache lebt in der Bridge.
- **Abstractions**: die public Seams (für IO und Downstream-Slices). **TestApis/Fakes**: die
  Black-Box-Test-Fläche.

## `shared_kernel` bekommt die Outbox

> **Verfeinert durch [ADR-0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md):**
> Der hier als *ein* Kernel skizzierte Aufbau ist in **zwei** Crates geschnitten —
> `shared_kernel` (io-facing Outbox-Seams) + `application_kernel` (slice-interne
> Handler-Konventionen) — weil Rust kein cross-crate `internal` kennt, sodass `io` die
> slice-internen Verträge nicht sieht. Die C#-nahen Vertragsnamen unten
> (`HandleOutcome`/`Severity`/`add_to_outbox`/`OutboxHandlerBase`/`message_type()`) sind
> Rust-idiomatisch ersetzt (`Result<(), OutboxError>`/`publish`/`const MESSAGE_TYPE`/
> `TypedOutboxHandler`). Der Text unten bleibt als Historie unverändert.

Der Shared-Kernel bleibt dünn und interface-only, wird aber um die **Outbox-Verträge**
erweitert (Vorbild `examples/Common/Abstractions` + `examples/Outbox/Adapter`):

- Handler-Ebene: `Handler`, `Command`, `PublishCommand`, `Publisher`, `Response`.
- Outbox-Ebene: `OutboxMessage` (Envelope: Id/Type/PartitionKey/Content/Zeit/Attempt),
  `OutboxCommand` (Wire-Marker mit `message_type` + `partition_key`), `OutboxPublisher`
  (`add_to_outbox`), `OutboxHandler` (`handle(OutboxMessage) -> HandleOutcome`),
  `HandleOutcome` (`Handled`/`Retry`/`Faulted`), `Severity`, `UnitOfWork` (atomarer Commit),
  `OutboxHandlerBase`-Äquivalent, eine JSON-Konvention.

**Envelope-Konvention (Zeiger vs. Message-Identität):** der **Aggregat-Zeiger** reist im
`partition_key` (= Aggregat-Identität), die `id` ist die io-vergebene **Message-UUID**.
Ein ID-Zeiger-Consumer — etwa `obfuskieren`, der die `Ingested`-Nachricht per Aggregat-ID im
einlesen-Speicher auflöst — liest deshalb den `partition_key`, **nie** die `id` (die pro
Nachricht eindeutig ist und den Lookup verfehlte). Produzenten verlassen sich spiegelbildlich
nur auf den `partition_key`, nie darauf, dass die io-`id` einen fachlichen Wert trägt.

Diese Verträge sind **Adapter-Niveau** — die blinde Domäne nutzt sie nie (ADR-0007).

## Datenschutz-Grenze an `obfuskieren` (nicht am Eingang)

Die Daten **müssen** ins System; an einem Punkt liegen sie lokal **un-obfuskiert** —
unvermeidlich und **kein Unterschied zur lokal liegenden `.csv`**. Roh darf darum **lokal
existieren und persistieren**: die `einlesen`-Tabelle und die `einlesen → obfuskieren`-
Message tragen rohe Werte. **`obfuskieren` ist die Grenze**; ab dort laufen nur Token, und
roh **verlässt die Maschine nie** (LLM/Report).

Konsequenz für den compilergeprüften Beleg: die **`assert_not_impl_any!`-„kein
`Serialize`"-Sperre wandert** von den rohen Newtypes (`Iban`/`Bic`) auf die **Egress-DTOs** —
die Kategorisierer-Anfrage ans LLM und der Report-Output. Diese dürfen **keinen** rohen
Identifikator tragen. Der frühere Beleg „`Iban` leitet nie `Serialize` ab" ist damit **zu
streng** und wird ersetzt: `Iban` **muss** für den lokalen `einlesen → obfuskieren`-Hop
serialisierbar sein.

## Testbarkeit jetzt, echtes IO später

Handler, Adapter (Consume-/Publisher-/Port-Bridges) und TestApis sind **vollständig
plan-/bau-/testbar** ohne echtes IO: die `_specs` fahren jeden Slice als Black-Box über seine
Fluent-TestApi mit **Fakes** (Fake-`OutboxPublisher`, Fake-Stores, Fake-Quell-Repository).
Das echte IO-Crate (SQLite-Outbox + -Stores, Datei/CSV-Parse) kommt **später** und erfüllt
dieselben Seams.

## Was dieses ADR supersedet / verfeinert

- **ADR-0007, „Obfuskierung als eigener Slice" + „Slice→Slice über Bridge":** Der
  `einlesen`-Slice konsumiert `obfuskierung` **nicht** inline „bevor seine Domäne etwas
  sieht". `obfuskieren` ist ein **nachgelagerter Consumer**; die Kopplung ist die
  Outbox-Message, nicht ein synchroner Bridge-Aufruf.
- **ADR-0003/0001/0007, Datenschutz-Verortung:** nicht „roh ist nie serialisierbar", sondern
  „roh verlässt die Maschine nie" — Grenze an `obfuskieren`, Compiler-Beleg auf den
  Egress-DTOs (s. o.). Das Ziel (kein realer Name/Identifikator zum LLM/Report) bleibt.
- **ADR-0002, „Persistenz speichert DTOs, nicht Domänenobjekte":** präzisiert. Die **lokale
  `einlesen`-Persistenz** (Roh-Tabelle) und die `einlesen → obfuskieren`-Message dürfen
  **rohe Werte** tragen — **kein Egress** (kein Unterschied zur lokal liegenden `.csv`). Die
  DTO-only-Regel gilt unverändert für alles, was die **Maschine verlässt** (LLM/Report), und
  für die eigentliche `Speicher`-Persistenz der auswertungsrelevanten DTOs.
- **ADR-0007, „Datenfluss"/Feld-Semantik:** der Handler arbeitet auf Domänen-Typen und
  orchestriert das Aggregat; Command/Response/Error sind Domänen-Typen, die
  Außenwelt-Anpassung (Message↔Domäne) lebt in der Consume-Bridge. Die Verortung von
  Feld-Parsern (deutscher Betrag etc.) folgt separat dem Lackmustest „fachliche Entscheidung →
  Domäne, sonst Adapter".
- **Slice-Umbenennung:** `csv_camt_v8` → **`einlesen`** (Verantwortung, nicht Format) —
  **ausgeführt**: Crates, Code und `CLAUDE.md` führen den `einlesen`-Slice.

## Bewusst vertagt

Das echte `io`-Crate (SQLite-Outbox-Broker + -Stores, Datei/CSV-Parse), die
`report`-Presenter-Crate und der `auswerten`-Slice (mit `Kategorisierer`-Port ans LLM-Orakel)
bleiben Zielbild; zuerst der `einlesen`-Slice nach diesem Muster, dann `obfuskieren`.
