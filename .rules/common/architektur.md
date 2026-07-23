# Architektur-Prinzipien

Maßgeblich: [ADR-0001](../../docs/adr/0001-hexagonale-architektur.md),
[ADR-0004](../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md),
[ADR-0008](../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md),
[ADR-0011](../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
[ADR-0012](../../docs/adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md),
[ADR-0013](../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md). Die
crate-weite Durchsetzung steht in
[crates/dependencies-und-kapselung.md](../crates/dependencies-und-kapselung.md).

## Externe Information hinter einem Port

Die Domäne kennt keine Formate, Anbieter, Speicher oder das LLM — jede externe Information
sitzt hinter einem Port (Trait), erfüllt im `io`-Layer. Seiteneffekte nur an den Rändern.

Do:

```rust
// einlesen_domain: die Domäne verlangt nur den Port
pub trait RawLineSource {
    fn fetch(&self) -> Result<Vec<RawLine>, SourceError>;
}
// io: erfüllt ihn mit dem konkreten csv-camt-v8-Adapter
```

Don't:

```rust
// einlesen_domain
use csv::Reader; // die Domäne zieht ein I/O-Crate und kann plötzlich I/O
```

## Die Domäne ist die Analyse; das LLM ist nur ein Orakel

Die reine Domäne rechnet die Analyse selbst. Das LLM wird ausschließlich zu **unbekannten**
Fingerabdrücken gefragt. Lackmustest: ohne LLM bleibt das Programm wertvoll.

Do:

```rust
let cashflow = fenster.buchungen().iter().fold(Betrag::null(), Betrag::add);
if !register.kennt(&fingerabdruck) {
    kategorisierer.frage(KategorieAnfrage::new(fingerabdruck, text))?; // nur das Unbekannte
}
```

Don't:

```rust
let kostentreiber = llm.frage("Was sind die größten Kostentreiber?")?; // Domäne kapituliert
```

## Kopplung nur über die Outbox — kein synchroner Folgeaufruf

Jeder Pipeline-Schritt ist ein Outbox-Consumer: er publiziert eine Nachricht für den
nächsten Schritt, statt ihn aufzurufen. `io` klammert lesen/ablegen/publizieren in **eine**
Transaktion (atomar, at-least-once, resümierbar); der Slice bleibt transaktions-blind.

Do:

```rust
fn handle(&self, cmd: ObfuscateBooking) -> Result<(), ObfuscationError> {
    let raw = self.source.read(cmd.id)?;                   // lesen
    let id = self.store.save(obfuscate(raw, &self.seed))?; // ablegen
    self.publisher.publish(Obfuscated { id })?;            // Zeiger publizieren
    Ok(())
}
```

Don't:

```rust
let id = self.store.save(obf)?;
auswerten_slice.evaluate(id)?; // synchroner Folgeaufruf statt Outbox-Nachricht
```

## Durchweg terminal — kein Retry, kein Dead-Letter im Slice

Alle drei I/O-Schritte sind fehlbar. Ein Fehler ist terminal: der Slice gibt seinen
**nackten, redigierten** Domänen-Fehler als reine `?`-Kette zurück, ohne Klassifikation.

Do:

```rust
let raw = self.source.read(cmd.id)?; // ? — terminal, io surfacet den Fehlschlag
```

Don't:

```rust
for _ in 0..3 { // Retry-Schleife im Slice
    match self.source.read(cmd.id) {
        Ok(r) => return self.weiter(r),
        Err(_) => continue, // Dead-Letter/Retry ist nicht Sache des Slice
    }
}
```

## Uniformes ID-Handoff

Jeder Slice liest den Vorgänger **per ID**, schreibt in seine **eigene** neue Tabelle (die
Persistenz vergibt die neue ID) und publiziert **diese neue** ID als Zeiger. Aggregate
referenzieren Fremd-Entitäten nur per ID, nie als eingebettetes Objekt.

Do:

```rust
let raw = self.store.read(cmd.id)?;        // Vorgänger per ID
let neue_id = self.store.save(aggregat)?;  // eigene Tabelle vergibt die ID
self.publisher.publish(Obfuscated { id: neue_id })?; // die neue ID publizieren
```

Don't:

```rust
self.publisher.publish(Obfuscated { raw_booking: raw })?; // Fremd-Aggregat eingebettet statt per ID
```
