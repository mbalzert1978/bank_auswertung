---
id: "0015"
title: "einlesen-Slice als Pipeline-Consumer (Umbau von csv_camt_v8)"
status: closed
---

# einlesen-Slice als Pipeline-Consumer (Umbau von csv_camt_v8)

**Typ:** AFK

> Neu aus [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md). Baut die
> Architektur des **erledigten** [0003](0003-csv-camt-v8-adapter-zu-buchung.md) nach
> dem event-getriebenen Pipeline-Muster **neu** — der fachliche Inhalt (Mapping,
> einheitliche Währung, Fail-safe-Empfänger, dynamischer Zeitraum) bleibt, der
> Schnitt (Handler auf Domänen-Typen + Bridges + Outbox) ändert sich. **Kein**
> Inline-Obfuskieren mehr — das ist der nachgelagerte Slice
> [0016](0016-obfuskieren-slice-als-pipeline-consumer.md).

## Was zu bauen ist

Der **Kopf der Pipeline** als Consumer-Slice-Triade `einlesen_domain`/`_adapter`/
`_specs`. Er heißt nach seiner **Verantwortung**, nicht nach dem Format: CSV-CAMT-v8
ist nur **eine Eingabeform** hinter dem Quell-Seam (XML-camt/MT940 später = weiterer
IO-Adapter, kein neuer Slice).

- **Handler** (crate-intern) auf **Domänen-Typen**: bekommt ein eingehendes Command
  (der ganze Batch, nicht zeilenweise) + seine Domänen-Ports (Quell-Repository) und
  einen `Publisher` **injiziert**, spricht das Repository an, **prüft/validiert** die
  Rohdaten und orchestriert das Aggregat. `Command`/`Response`/`Error` sind interne
  Domänen-Typen — kein public IO weiß von ihnen.
- **Adapter** = Anpassung an die Außenwelt: **Consume-Bridge** (`OutboxHandlerBase`-
  Muster: `OutboxMessage` → Domänen-Command → Handler → `HandleOutcome`),
  **Publisher-Bridge** (Domänen-Command → Wire-DTO `OutboxCommand` → Outbox) und
  **Port-Bridges** (Quell-Seam für die geparsten Zeilen). **Abstractions** (public
  Seams für IO) + **TestApi/Fakes**.
- **Roh darf lokal erfasst werden:** der Slice schreibt seine **Roh-Tabelle** und —
  in **einer** Transaktion (`UnitOfWork`) — eine **Outbox-Message `Eingelesen`** für
  den nächsten Schritt. Das ist **kein Egress** (kein Unterschied zur lokal liegenden
  `.csv`); die Datenschutz-Grenze ist erst `obfuskieren` (Slice 0016).
- Byte-Level-CSV-Parsen (Delimiter/Quoting/Kopfzeile, `csv`-Crate) bleibt im vertagten
  `io`; der Quell-Seam liefert bereits geparste Zeilen-DTOs. Einheitliche Währung über
  die Datei wird zugesichert; der Zeitraum ergibt sich dynamisch aus den Daten.

## Abnahmekriterien

- [ ] Der Slice liegt als Triade `einlesen_domain`/`_adapter`/`_specs` vor; der
      Handler nutzt `shared_kernel::Handler` mit assoziierten `Command`/`Response`/
      `Error` (Domänen-Typen, kein public IO).
- [ ] Consume-Bridge deserialisiert `OutboxMessage.content` → Domänen-Command,
      dispatcht an den Handler und mappt `Response`/`Error` → `HandleOutcome`.
- [ ] Der Handler schreibt Roh-Erfassung **und** die Outbox-Message `Eingelesen` in
      **einer** `UnitOfWork` (atomar, at-least-once) — über einen Fake-`OutboxPublisher`
      im Test belegt.
- [ ] Fachlicher Kern erhalten: jede Zeile → gültige Erfassung; Fail-safe-Empfänger;
      gemischte Währungen ergeben einen klaren Fehler statt Falschsummierung;
      dynamischer Zeitraum.
- [ ] **Kein Inline-Obfuskieren** im `einlesen`-Slice — Token-Vergabe/Scrubbing sind
      nicht Teil dieses Slice (gehören zu 0016).
- [ ] `_specs` fahren den Slice als **Black-Box** über die Fluent-TestApi mit Fakes
      (Fake-Quell-Repository, Fake-`OutboxPublisher`) — kein Zugriff auf Interna, kein
      echtes IO.
- [ ] Der `architecture`-Test bleibt grün (keine `adapter→adapter`-Kante; Slice→Slice
      nur über injizierte `shared_kernel`-Interfaces).
- [ ] `cargo build`/`test`/`clippy --all-targets` und `cargo fmt --check` grün.

## Hinweis zur Umbenennung

Die Slice-Umbenennung `csv_camt_v8` → `einlesen` ist **beschlossen** (ADR-0008), wird
aber **mit diesem Umbau** ausgeführt; bis dahin führen Crates/Code weiter
`csv_camt_v8`. Der obsolete uncommittete „Phase-G"-Zwischenstand des alten Adapters
wird in diesem Zuge abgebaut (`shared_kernel` bleibt).

## Blockiert durch

- Blockiert durch [0014](0014-shared-kernel-outbox-vertraege.md) (Outbox-Verträge im
  `shared_kernel`).
- Baut auf dem erledigten [0003](0003-csv-camt-v8-adapter-zu-buchung.md) auf. **Kein**
  Eigen/Fremd im `einlesen`-Slice: die Domäne trägt die rohe Gegen-IBAN nur unverändert
  mit, die `Gegenkonto::Eigen|Fremd`-Auflösung ist Sache des nachgelagerten `auswerten`
  (token-basiert; [Issue 0002](0002-eigene-konten-config-schema-und-loader.md) liefert nur
  noch das Config-Artefakt, der frühere `eigene_konten`-Slice ist entfernt).
