# Zwei Kernel-Crates + Rust-idiomatische Outbox-Verträge

Dieses ADR **verfeinert** den Kernel-Aspekt von
[ADR-0008](0008-event-getriebene-pipeline-und-outbox.md) (§ „`shared_kernel` bekommt die
Outbox") und das „ein dünner `shared_kernel`" aus
[ADR-0007](0007-modularer-monolith-feature-slices.md): der **eine** `shared_kernel` wird in
**zwei** Crates geschnitten, und die dort C#-nah skizzierten Verträge werden
**Rust-idiomatisch** ausformuliert. Die Pipeline-Aussagen von ADR-0008 (`einlesen →
obfuskieren → auswerten → report`, Outbox-Pattern, Datenschutz-Grenze an `obfuskieren`)
bleiben **unberührt maßgeblich** — nur die Gestalt der Kernel-Verträge ändert sich.

## Problem

ADR-0008 skizzierte **einen** interface-only `shared_kernel` mit sowohl den
**Handler-Bau-Konventionen** (`Command`/`Handler`/`Publisher`/…) als auch den **Outbox-Seams**
(`OutboxHandler`/`OutboxPublisher`/…), C#-nah benannt
(`HandleOutcome`/`Severity`/`add_to_outbox`/`OutboxHandlerBase`, `message_type()`). Bei der
Umsetzung (Issue 0014) traten zwei Punkte hervor:

1. **Sichtbarkeit vs. Zielgruppe.** In C# liegen die Slices in *einer* Assembly; `internal`
   hält ein Interface assembly-weit sichtbar, aber vor fremden Assemblies verborgen. Rust
   kennt **kein** cross-crate `internal` — `pub(crate)` endet an der Crate-Grenze, `pub` ist
   für **jeden** sichtbar. Da ein Slice-Adapter (eigene Crate) die Verträge nutzt, **müssen**
   sie `pub` sein — und dann sähe auch das (spätere) `io`-Crate die **slice-internen**
   Handler-Konventionen, die es nichts angehen. Ein Kapselungs-Konflikt, den der
   Dependency-Richtungs-Test nicht abfängt (er prüft Crate-**Kanten**, nicht Symbol-Nutzung
   *innerhalb* einer erlaubten Kante).
2. **Idiomatik.** Die C#-nahen Namen sind in Rust untypisch: C# hat kein `Result`, daher
   `HandleOutcome` + `Severity`; „Base" ist OOP-Vererbung; ein typgebundener Konstant-Schlüssel
   ist eine assoziierte Konstante, keine Methode.

## Entscheidung A — Rust-idiomatische Verträge

- **`Result<(), OutboxError>` statt `HandleOutcome` + `Severity`.** `Result` trägt
  Erfolg/Misserfolg selbst; `Handled` = `Ok(())`, `Severity` → Fehler-Varianten.
  `OutboxError::Retry|Warning|Critical` steuert, was der Prozessor mit der Nachricht tut
  (Retry vs. Dead-Letter). Kernel bleibt dependency-frei — `Display`/`Error` von Hand, kein
  `thiserror`.
- **`publish` statt `add_to_outbox`** (`OutboxPublisher`).
- **`const MESSAGE_TYPE: &'static str` statt `fn message_type()`** (`PublishCommand`,
  `OutboxCommand`): der Dispatch-Key ist ein typgebundener, kompilierzeit-fester Wert, kein
  Verhalten. Der instanzabhängige `partition_key(&self)` bleibt Methode.
- **`TypedOutboxHandler` statt `OutboxHandlerBase`:** gemeint ist der *typisierte* Handler, aus
  dem der objekt-sichere `OutboxHandler` wird. **`map_outcome` entfällt** — `dispatch` gibt
  selbst ein `Result`, der Domänen-Fehler mappt über `Into<OutboxError>` (Adapter-Aufgabe).
- **`OutboxHandler` bleibt** eigener Trait — der **objekt-sichere** Dispatch-Seam
  (`dyn OutboxHandler`), über den der Prozessor heterogene Handler per `message_type` auflöst;
  `TypedOutboxHandler` kann das wegen seiner assoziierten Typen nicht.

## Entscheidung B — Zwei Kernel-Crates

> **Teilweise supersedet/geflippt durch [ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md):**
> Der **Zwei-Crate-Schnitt bleibt**, aber die **Zielgruppen kehren sich um**. Weil die generische
> `OutboxConsumer<H: Handler>` in `io` lebt, **muss `io` `application_kernel` sehen**. Danach ist
> `application_kernel` der **`io`-sichtbare Slice↔`io`-Vertrag** (`Command`/`Handler`/`Response`/
> `Publisher`/`PublishCommand` + neu `CommandDecoder`; bindet `_adapter` **und** `io`), und
> `shared_kernel` ist **komplett `io`-privat** (alle Outbox-Seams + `UnitOfWork`; bindet nur `io`);
> der `<slice>_adapter` bindet **nur** `application_kernel`. `TypedOutboxHandler` zieht nach `io` /
> löst sich auf, wodurch `application_kernel` **dependency-frei** wird. Neuer Graph:
> `io → application_kernel`, `io → shared_kernel`, `<slice>_adapter → application_kernel`.

Kapselung über Crate-Grenzen wird in Rust durch den **Crate-Schnitt** ausgedrückt, nicht durch
Modifikatoren (Interface Segregation + Package-Cohesion). Der eine Kernel zerfällt entlang der
**Zielgruppe**:

- **`shared_kernel`** — die **io-facing** Outbox-Seams, die `io` sieht/erfüllt:
  `OutboxMessage`, `OutboxCommand`, `OutboxError`, `OutboxHandler`, `OutboxPublisher`,
  `UnitOfWork` (+ die `outbox_payload`-JSON-Konvention). Weiterhin dependency-frei (nur `std`).
- **`application_kernel`** — die **slice-internen** Handler-Bau-Konventionen, die nur die
  Slices sehen: `Command`, `PublishCommand`, `Handler`, `Publisher`, `TypedOutboxHandler`.
  Hängt an `shared_kernel` (die `TypedOutboxHandler`-Basis erzeugt einen
  `shared_kernel::OutboxHandler`).

Graph: `io → shared_kernel`; `<slice>_adapter → shared_kernel + application_kernel`;
`application_kernel → shared_kernel`. So kann `io` das `application_kernel` **gar nicht
importieren** — die slice-internen Verträge sind für `io` compiler-echt unsichtbar.

## Konsequenz — Default-Methode statt Blanket-Impl

Solange `OutboxHandler` und `TypedOutboxHandler` in **einer** Crate lagen, lieferte
`impl<T: TypedOutboxHandler> OutboxHandler for T` den `OutboxHandler` gratis. Nach dem Split
verbietet die **Orphan-Regel** (E0210) diesen Blanket-Impl (fremder Trait für generischen `T`).
Ersatz: `TypedOutboxHandler::handle_outbox` als **Default-Methode** — die Verdrahtung
(Envelope → Request → `Result`, leerer Inhalt → `OutboxError::Critical`) lebt weiter genau
**einmal**; der Slice-Adapter implementiert `OutboxHandler` für seinen **konkreten** Bridge-Typ
(lokaler Typ → erlaubt) und delegiert in **drei Zeilen** an `handle_outbox`. Nebeneffekt: die
frühere „entweder/oder"-Einschränkung des Blanket-Impls entfällt.

## Enforcement

> **Geflippt durch [ADR-0011](0011-slice-reine-pipeline-outbox-mechanik-in-io.md):**
> Die Regel „nur `_adapter` bindet `application_kernel`" wird **„`_adapter` **und** `io`
> binden `application_kernel`"**, und es kommt **„ein `<slice>_adapter` bindet **kein**
> `shared_kernel`"** (`adapter_bindet_kein_shared_kernel`) hinzu. Der PreToolUse-Hook bleibt
> weiter unverändert.

Der `architecture`-Test bekommt die Regel **„nur `_adapter` bindet `application_kernel`"**
(`io`/`_domain`/`_specs` würden rot); `shared_kernel` bleibt dependency-frei (bestehender
`interface_only`-Test). Der PreToolUse-Hook braucht **keine** Änderung — `path`-Deps sind
sanktioniert.

## Alternative (verworfen)

`TypedOutboxHandler` im `shared_kernel` belassen (Blanket-Impl behalten, `io` sähe ihn) und nur
die Handler-Ebene auslagern — spart die drei Delegations-Zeilen, verfehlt aber die
Kapselungs-Absicht (`io` sähe einen rein slice-internen Bau-Vertrag). Der volle Split wurde
gewählt.

## Was dieses ADR supersedet / verfeinert

- **ADR-0008, § „`shared_kernel` bekommt die Outbox":** der eine Kernel wird zwei
  (`shared_kernel` io-facing + `application_kernel` slice-intern); die C#-nahen Verträge werden
  Rust-idiomatisch (`Result`/`OutboxError`, `publish`, `const MESSAGE_TYPE`,
  `TypedOutboxHandler`).
- **ADR-0008, „Der Handler arbeitet auf Domänen-Typen":** das `Error → HandleOutcome`-Mapping
  der Consume-Bridge ist ein `Error → OutboxError`-Mapping über `Into<OutboxError>`; `handle`
  gibt `Result<(), OutboxError>`. Dass `Response`/`Error` am `Handler` keine Schranke tragen,
  bleibt.
- **ADR-0007, „ein dünner `shared_kernel`":** weiterhin dünn/interface-only, aber **zwei**
  Kernel-Crates entlang der io-Sichtbarkeit.
