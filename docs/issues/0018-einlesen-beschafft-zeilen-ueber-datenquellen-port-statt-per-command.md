---
id: "0018"
title: einlesen beschafft Zeilen über Datenquellen-Port statt per Command
status: open
---

# einlesen beschafft Zeilen über Datenquellen-Port statt per Command

**Typ:** AFK

> Behebt einen Architekturfehler an der Command-/Handler-Grenze des `einlesen`-Slice.
> Maßgeblich: [ADR-0008](../adr/0008-event-getriebene-pipeline-und-outbox.md)
> (event-getriebene Pipeline/Outbox) und
> [ADR-0009](../adr/0009-kernel-split-und-idiomatische-outbox-vertraege.md)
> (Kernel-Split & idiomatische Outbox-Verträge). Der Slice bleibt Kopf der Pipeline;
> geändert wird nur, **woher** der Handler die Rohzeilen bezieht.

## Was zu bauen ist

Heute reicht die `ReadTransactions`-Nachricht **den ganzen Batch** (alle Rohzeilen) als
Command-Nutzlast in den Slice; der Handler iteriert die im Command mitgelieferten Zeilen.
Das ist falsch: der Command ist der **Auslöser** der Pipeline, nicht der Transporter der
Rohdaten. Ein Batch als Nachrichten-Nutzlast bläht die Outbox-Zeile auf und koppelt die
Beschaffung an den Nachrichten-Envelope, statt sie hinter einen Port zu legen, den io
später erfüllt.

Stattdessen **beschafft der Handler die Rohzeilen selbst** über einen neu einzuführenden
**Datenquellen-Port**. Der Command trägt keine Zeilen-Nutzlast mehr, sondern ist ein
reiner **Auslöser** mit optionalem (vorerst leerem) Konfigurationsfeld für den Lauf.
Hinter dem Port sitzt später die konkrete io-Datenquelle (CSV-CAMT-v8 usw.); für dieses
Slice genügt der Port plus ein Fake für die Tests.

**Vertragsform (entschieden):**

- **Domäne setzt konkret an.** Ein von der Domäne verlangter Datenquellen-Port liefert den
  Batch als bereits domänen-typisierte Rohzeilen und ist **fehlbar**: gelingt die
  Beschaffung nicht, gibt er einen **domänen-typisierten** Beschaffungs-Fehler zurück
  (redigiert, nie ein Rohwert). Der Handler zieht den Batch aus diesem Port, deutet dann
  wie bisher je Zeile ein Aggregat, staged es und publiziert die Aggregat-ID als Zeiger.
- **Adapter setzt auf die Domäne um.** Eine Port-Bridge mappt „außen → innen" einen
  öffentlichen, roh-typisierten Datenquellen-Seam (den io erfüllt und der extern mit dem
  bestehenden Rohsatz-DTO spricht) auf den Domänen-Port. Die **Präsenz-Prüfung der
  Pflichtfelder** (heute in der Consume-Bridge vor dem Handler) **wandert hinter diesen
  Seam** in die neue Bridge und wird dort — wie ein io-Beschaffungsfehler — auf den
  domänen-typisierten Beschaffungs-Fehler abgebildet (redigiert, nur Feldname). Der
  Auslöser-Command wird aus dem entschlackten Wire-DTO (ohne Zeilen) gebaut.
- **Bestehende DTOs/VOs wiederverwenden**, soweit möglich (Rohsatz-DTO, Rohzeilen-VO);
  keine neuen Wire-Formen erfinden, wo eine bestehende trägt.

**Fehlerbehandlung (bewusst ohne Zusatz-Mechanik):** die neue Beschaffungs-/Präsenz-Fehler
und der bestehende Deutungs-Fehler bleiben **terminal** und bubblen wie heute zu
`OutboxError::Warning`; die **io-Outbox dead-lettert batch-atomar**. Die
**Dead-Letter-Queue ist der Fehler-Publish** — es wird **keine** separate Fehler-Nachricht
eingeführt (keine Redundanz). Die Redaktion (nur Feldname/Fehlerart, nie ein Rohwert)
bleibt bis in die Dead-Letter-Meldung erhalten. Fan-out (eine „Eingelesen"-Nachricht je
gültiger Zeile), Batch-Atomarität und Zeiger-Semantik (Tabelle = Wahrheit, Message =
Pointer) bleiben unverändert.

**Schichtung wahren:** der Handler hängt nur an von der Domäne verlangten Ports; der neue
Datenquellen-Port ist eine Domänen-Abstraktion, die der Adapter bindet — kein
Domänen-Re-Export, keine io-Abhängigkeit in der Domäne, kein neuer externer
`[dependencies]`-Eintrag außerhalb `io`.

**Tests bleiben Black-Box** über die Fluent-TestApi: die Rohzeilen-Arrangierung speist
künftig den **Fake-Datenquelle** hinter dem neuen Port statt der Nachrichten-Nutzlast; die
Kette läuft real gegen Fakes. Fan-out-, Redaktions- und Ablehnungs-Szenarien
(fehlendes Pflichtfeld → batch-atomarer Dead-Letter; unbekannte Währung) bleiben grün.

**Nicht in diesem Slice:** keine reale io-/CSV-/SQLite-Implementierung (bleibt deferred) —
nur Port + Fake. Keine anderen Slices anfassen, außer der Vertrag zwingt dazu (dann vorab
benennen). Command-Umbau nicht über das Nötige hinaus ausweiten.

**Vor der Code-Änderung:** kurzen Umsetzungsplan vorlegen (Name/Signatur des neuen Ports,
neue Command-Form, betroffene Dateien, Testanpassung) und jede offene Detailfrage
markieren.

## Abnahmekriterien

- [ ] Der `einlesen`-Command trägt **keine** Zeilen-Nutzlast mehr, sondern ist ein
      Auslöser mit optionalem, vorerst leerem Konfigurationsfeld; auch das Wire-DTO trägt
      keine Zeilen mehr.
- [ ] Der Handler bezieht die Rohzeilen über den neuen, von der Domäne verlangten
      Datenquellen-Port; die Verdrahtung an die Außenwelt (Fake, später io) besorgt eine
      Adapter-Port-Bridge.
- [ ] Der Datenquellen-Port ist fehlbar und gibt bei misslungener Beschaffung einen
      domänen-typisierten, **redigierten** Fehler zurück; die Präsenz-Prüfung der
      Pflichtfelder liegt hinter dem Seam und wird auf diesen Fehler abgebildet.
- [ ] Beschaffungs-/Präsenz- und Deutungs-Fehler bubblen terminal zu
      `OutboxError::Warning` (Dead-Letter batch-atomar); **keine** separate
      Fehler-Nachricht wird eingeführt; die Redaktion trägt bis in die Dead-Letter-Meldung
      (kein Rohwert).
- [ ] Fan-out (eine „Eingelesen"-Nachricht je gültiger Zeile) und Zeiger-Semantik bleiben
      unverändert.
- [ ] Die Slice-Specs bleiben Black-Box über die Fluent-TestApi; die Rohzeilen speisen den
      Fake hinter dem Port; die bestehenden Ablehnungs-Szenarien (fehlendes Pflichtfeld →
      batch-atomarer Dead-Letter; unbekannte Währung) bleiben grün.
- [ ] `cargo build` / `cargo test` / `cargo clippy --all-targets` / `cargo fmt --check`
      **und** `cargo test -p architecture` sind grün.

## Blockiert durch

- None - can start immediately.
