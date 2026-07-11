# Der Vertical Slice ist eine reine Pipeline — die Outbox-Mechanik lebt in `io`

Dieses ADR **verfeinert** [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)
(§ „Der Adapter ist die Anpassung an die Außenwelt", „Der Handler arbeitet auf
Domänen-Typen") und **verfeinert/supersedet in Teilen**
[ADR-0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md) (Entscheidung B +
Enforcement): der Vertical Slice wird eine **reine `Command → Domäne → Response`-Pipeline
ohne jede Outbox-Mechanik**; die Consumer-Rolle, die Retry/Dead-Letter-Taktik, die
`UnitOfWork` und die Envelope-Kenntnis leben **vollständig in `io`**. Die
**Pipeline-Garantien von ADR-0008** (`einlesen → obfuskieren → auswerten → report`,
Outbox-Pattern: atomar, at-least-once, wiederaufsetzbar; Datenschutz-Grenze an
`obfuskieren`) bleiben **unberührt maßgeblich** — nur die Verortung der Outbox-Mechanik
und der Kernel-Schnitt ändern sich.

Herkunft: Wayfinder-Karte
[#8](https://github.com/mbalzert1978/bank_auswertung/issues/8), Grilling-Tickets
[#9](https://github.com/mbalzert1978/bank_auswertung/issues/9)–[#12](https://github.com/mbalzert1978/bank_auswertung/issues/12).

## Problem

Die **Domäne war schon sauber**: `einlesen_domain::ReadHandler` ist bereits die reine
Pipeline (beschaffen → deuten → staged speichern → id publizieren, Rückgabe des eigenen
`ReadError`). Der Leak saß im **`_adapter`-Rand + den Kernel-Verträgen**:

1. **Consumer-Rolle im Slice.** Die `ConsumeBridge` **ist** `impl shared_kernel::OutboxHandler`
   **und** entscheidet die Taktik (`ReadError → OutboxError::Warning`). Outbox-Rolle +
   Dead-Letter-Wissen — Infrastruktur — stecken mitten im Slice-Adapter.
2. **Envelope-Zwang beim Command.** Der Command ist ein **feldloser Auslöser**, weil die
   Envelope-Konvention (`partition_key`) die Daten trägt — „leer sein **müssen**" statt
   „leer sein **dürfen**".
3. **Kernel-Kopplung falsch herum.** Nach ADR-0009 bindet der `<slice>_adapter`
   `shared_kernel` (die io-facing Outbox-Seams) direkt — genau die Mechanik, die er nicht
   kennen sollte.

Ein Vertical Slice ist eine Pipeline aus *eingehendem Command → Validierung → Domänen-Arbeit
→ Response*, mit Mapping extern↔intern (DTO/Entity) an den Rändern — **das hat mit der Outbox
nichts zu tun**. Die Outbox ist der **Transport zwischen** Slices, kein Teil eines Slice
(*Auflösung A*): sie bleibt, nur der Slice wird von ihr entkoppelt.

## Entscheidung A — Grenze: der Slice ist eine reine Pipeline, `io` ist der Consumer

- Der Slice behält seine **eigenen Domänen-Ports** (`Store`, `EventSink`), staged beide und
  gibt weiter `Result<(), DomänenFehler>` zurück. Die „Response" ist **kein** Rückgabewert,
  sondern der Seiteneffekt über den Domänen-Port — so bleibt das Paar `store`+`publish` **im
  Handler** und die ADR-0008-Atomarität hält natürlich, die id-Vergabe bleibt lokal.
  (Verworfen: „reine Funktion", die die Aggregate zurückgibt und `io` speichern/publishen
  lässt — hätte id-Vergabe + Fan-out nach `io` gezogen und das atomare Paar zerrissen.)
- Statt `ConsumeBridge: impl OutboxHandler` **pro Slice** implementiert **eine generische
  `OutboxConsumer<H: Handler>` in `io`** den `shared_kernel::OutboxHandler` — über den
  `application_kernel::Handler`-Port, den jeder Slice ohnehin erfüllt. Der Slice-Adapter
  implementiert `OutboxHandler` **nicht mehr**.
- **`io` besitzt die eine `UnitOfWork`.** Der Handler bleibt **transaktions-blind**: die
  `OutboxConsumer` öffnet die Transaktion, ruft `handle()` (staged) und committet **einmal** —
  **Consume-Ack + Aggregate + erzeugte Nachrichten atomar**. Daraus fallen at-least-once +
  wiederaufsetzbar. Zwang: `io` bindet den Publisher des Slice an **denselben**
  Transaktionskontext, den es committet.
- `application_kernel::TypedOutboxHandler` verliert seinen Zweck als slice-interne Bau-Basis
  und **zieht nach `io` / löst sich auf** (die Verdrahtung Envelope → Request → `Result` lebt
  dann einmal in der `OutboxConsumer`).

## Entscheidung B — Fehler-Taktik: jeder Handler-Fehler ist terminal

- **Jeder Handler-Fehler ist terminal → Dead-Letter.** Ein Fehler aus `handle()` bedeutet
  kaputte Daten / gebrochenes Invariant, nie „warte kurz". Der Slice gibt seinen **nackten
  Domänen-Fehler** zurück (Bild 2) und baut **kein** `OutboxError` mehr.
- **`io` reicht keinen neuen Typ durch.** Die generische `OutboxConsumer<H>` kennt `H::Error`
  konkret (monomorphisiert); der Bound `application_kernel::Handler::Error:
  std::error::Error` genügt. `io` leitet das **`Display`** als Dead-Letter-Grund weiter —
  **nie `Debug`**. Kein `Box<dyn Error>`, kein Kernel-Fehlertyp im Handler-Vertrag.
- **`OutboxError` bleibt `io`-intern** (`io` baut ihn): Handler-Fehler → `Warning` (terminal);
  `Critical` für die Envelope-/Dispatch-Ebene (leerer/ungültiger Inhalt); `Retry` **nur**
  `io`-intern.
- **Retry lebt zu 100 % in `io`**, auf zwei Ebenen: (a) Broker/Commit-Retry, (b) **hinter den
  Lese-Seams** — der `io`-Adapter wiederholt eine *kurz* gesperrte Datei/DB selbst. Ein
  Domänen-„unavailable"/„absent" bedeutet damit **permanente** Unerreichbarkeit → terminal;
  der Handler-Fehlerkanal ist luftdicht uniform-terminal.
- **Redaktion gewahrt** (ADR-0008-Datenschutz): der Slice-Fehler ist alleiniger Eigentümer
  des `Display`-Satzes (Feldname/io-Diagnose, nie ein Rohwert); `io` spiegelt ihn nur.

## Entscheidung C — Command trägt Nutzdaten; ein `CommandDecoder` mappt die Envelope

- Der eingehende Command ist ein **selbst-ständiger, getypter Daten-Transporter**
  (Mediator-/Dispatcher-Muster: `command<T>` trägt, was der Handler braucht —
  `ObfuscateBooking { id }`). Die Outbox-Envelope ist **reiner Transport**; der Command ist
  **nicht mehr durch sie zum leeren Trigger gezwungen**. `einlesen`s `ReadTransactions`
  **darf** leer bleiben (Config kommt im `io`-Schritt), aber nicht **weil** die Envelope die
  Daten trägt.
- Am Slice-Eingang sitzt ein **slice-gelieferter `CommandDecoder`** (reine ACL). **`io`
  extrahiert die Envelope** und reicht dem Decoder nur **nackte Strings**
  (`partition_key`/`content`) — der Slice-Adapter nennt am Eingang **keinen** Outbox-Typ
  (Shape Y):

  ```rust
  trait CommandDecoder {
      type Command;
      fn decode(partition_key: &str, content: &str) -> Option<Self::Command>;
  }
  ```

- **serde bleibt draußen**, solange kein Command echte `content`-Nutzlast trägt (heute:
  `obfuskieren` liest die id aus `partition_key`, `einlesen` ist leer). Erst eine echte
  JSON-Nutzlast bringt serde in den **Adapter** (sanktioniert), nie in die Domäne.

## Entscheidung D — Kernel-Schnitt geflippt: `application_kernel` ist der Slice↔`io`-Vertrag

ADR-0009 schnitt zwei Kernel-Crates entlang der **Zielgruppe** und machte
`application_kernel` **`io`-unsichtbar** (`io → shared_kernel`;
`<slice>_adapter → shared_kernel + application_kernel`). Entscheidung A **erzwingt den Flip**:
weil `OutboxConsumer<H: Handler>` in `io` lebt, **muss `io` `application_kernel::Handler`
sehen**. Der Schnitt kehrt sich um:

| Crate | Rolle | Wer bindet |
| --- | --- | --- |
| `application_kernel` | der **`io`-sichtbare Slice↔`io`-Vertrag**: `Command`, `Handler`, `Response`, `Publisher`, `PublishCommand` + neu `CommandDecoder` | `<slice>_adapter` **und** `io` |
| `shared_kernel` | **komplett `io`-privat**: `OutboxMessage`, `OutboxCommand`, `OutboxError`, `OutboxHandler`, `OutboxPublisher`, `UnitOfWork` | **nur** `io` |
| `<slice>_adapter` | Slice-Rand | **nur** `application_kernel`, **kein** `shared_kernel` |

Damit ist der Slice-Adapter **restlos outbox-frei**. Der Ausgangs-Bridge bleibt im
Slice-Adapter (weiter ACL), zielt aber auf `application_kernel::Publisher` statt
`shared_kernel::OutboxPublisher`; **`io` macht den letzten Outbox-Hop**
(`application_kernel::PublishCommand → shared_kernel::OutboxCommand/OutboxMessage`). Da
`TypedOutboxHandler` — der einzige Grund der ADR-0009-Kante `application_kernel →
shared_kernel` — nach `io` zieht, wird `application_kernel` **dependency-frei**; die zwei
Kernel werden Geschwister, `io` bindet beide.

Graph nach dem Flip: `io → application_kernel`, `io → shared_kernel`;
`<slice>_adapter → application_kernel`. So kann ein Slice-Adapter die Outbox-Mechanik
**gar nicht importieren** — sie ist für ihn compiler-echt unsichtbar.

## Enforcement

Die **verbotene Menge** (`OutboxHandler`/`OutboxError`/`OutboxMessage`) sind Symbole; die
bestehenden Mechanismen sind crate-level. Statt eines Symbol-Checks macht der Flip die Regel
wieder zur **Crate-Kante**:

- **`architecture`-Test** (`cargo_metadata`) trägt die neuen Kanten-Regeln — zwei Änderungen an
  `dependency_direction.rs`:
  1. **Neu:** `adapter_bindet_kein_shared_kernel` — ein `<slice>_adapter` hängt nicht an
     `shared_kernel` (der ganze Bann wird **eine Crate-Kante**).
  2. **Gelockert:** die ADR-0009-Regel „nur `_adapter` bindet `application_kernel`" wird
     **„`_adapter` **und** `io` binden `application_kernel`"**.
- **PreToolUse-Hook** (`enforce-dependency-direction.py`): **unverändert** — er regelt externe
  Fremd-Crates in `Cargo.toml`; die neuen Regeln sind interne Workspace-Kanten, die der Test
  trägt.
- **Specs-Regel** (`SPEC_ZUSATZ_DEPS = ["shared_kernel"]`): wird beim Umbau **mitgezogen** —
  die Black-Box-Specs assertieren dann gegen den `application_kernel`-Vertrag statt gegen die
  `shared_kernel`-Outbox-Typen.

## Alternative (verworfen)

- **`shared_kernel` splitten** (producer-seam `OutboxPublisher`/`OutboxCommand` adapter-facing
  vs. consumer-seam `OutboxHandler`/`OutboxError`/`OutboxMessage` io-facing), um den
  Symbol-Bann zur Crate-Kante zu machen. **Unnötig**, sobald der Adapter `shared_kernel` gar
  nicht mehr berührt (Entscheidung D) — er braucht auch die producer-seam nicht, weil der
  Ausgang über `application_kernel::Publisher` läuft.
- **Symbol-Grep** auf `<slice>_adapter/src` nach verbotenen `use`-Statements — text-basiert,
  außerhalb des `cargo_metadata`-Modells, brüchig; verfehlt die etablierte Linie „Kapselung =
  Crate-Schnitt".
- **Taktik im Slice behalten** (`HandleOutcome`/`Into<OutboxError>` wie die C#-Referenz) — hält
  die Consumer-Mechanik im Slice; Entscheidung A/B ziehen sie bewusst nach `io`. Die C#-Referenz
  darf das, weil dort **alles eine Assembly mit `internal`** ist; eine eigene Crate nicht.

## Was dieses ADR supersedet / verfeinert

- **ADR-0008, § „Der Adapter ist die Anpassung an die Außenwelt":** die **Consume-Bridge**
  wandert aus dem Slice in die generische `OutboxConsumer<H>` in `io`; die **Publisher-Bridge**
  bleibt im Adapter, zielt aber auf `application_kernel::Publisher` (io macht den Outbox-Hop).
- **ADR-0008, § „Der Handler arbeitet auf Domänen-Typen" (inkl. Issue-0018-Banner):** der
  Command ist ein **selbst-ständiger Daten-Transporter** (nicht mehr envelope-gezwungen leer);
  ein `CommandDecoder` über extrahierte Strings mappt Envelope → Command.
- **ADR-0009, Entscheidung B + Enforcement:** der Kernel-Schnitt wird **geflippt** —
  `application_kernel` `io`-sichtbarer Slice↔`io`-Vertrag (bindet `_adapter` **und** `io`),
  `shared_kernel` komplett `io`-privat (bindet nur `io`), `<slice>_adapter` bindet nur
  `application_kernel`. `TypedOutboxHandler` zieht nach `io` / löst sich auf; `application_kernel`
  wird dependency-frei.
- **ADR-0008/0009, Fehler-Mapping:** das `Error → OutboxError`-Mapping der Consume-Bridge
  **entfällt im Slice**; der Slice gibt seinen nackten Domänen-Fehler zurück, `io` mappt (alles
  terminal, Retry `io`-intern). `OutboxError` bleibt bestehen, aber **`io`-intern**.

## Bewusst vertagt

- **Der eigentliche Code-Umbau** — dieses ADR ist der **entschiedene Plan**, keine Code-Änderung.
  Der Umbau (den `shared_kernel`↔`application_kernel`-Schnitt real ziehen, `OutboxConsumer<H>` +
  `CommandDecoder` bauen, die Bridges umhängen, die Specs-Regel mitziehen) ist ein Folge-Effort.
- **Konkrete `RunConfiguration`-Feldinhalte** (Quellpfad/Format) — entscheidet erst der
  `io`-Schritt.
- Ob die Adapter-Fläche einen echten JSON-`content` je braucht (dann serde im Adapter) — erst
  wenn ein Command Nutzlast über die Aggregat-id hinaus trägt.
