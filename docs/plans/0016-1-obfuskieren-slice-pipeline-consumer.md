# Plan 0016-1 — obfuskieren-Slice als Pipeline-Consumer

Umsetzung von [Issue 0016](../issues/0016-obfuskieren-slice-als-pipeline-consumer.md).
Vorlage für Struktur/Schichtung/Testkultur: der fertige `einlesen`-Slice.

> **Achtung — dieser Plan verfeinert/weicht bewusst vom Issue-Wortlaut ab** (durch
> Nutzer-Entscheidungen in der Planungssitzung). Die abweichenden Punkte sind unten unter
> „Abweichungen vom Issue" gesammelt; Issue 0016 + ADR-0006 sind entsprechend nachzuziehen.

## Vorab geklärt (Guards)

- **0015 verfügbar?** Ja — `einlesen` ist gebaut und emittiert die konsumierte Message.
  Prosa „Eingelesen" = **Code `Ingested`** (Sprachkonvention). Dispatch-Key `"Ingested"`,
  camelCase-Wire-Felder aus `einlesen_adapter::IngestedMessage`.
- Issue-Prosa `shared_kernel::Handler`/`HandleOutcome` ist Vor-0015-Stand; **as-built** =
  konkreter Domänen-Handler + `application_kernel::TypedOutboxHandler` +
  `shared_kernel::OutboxError`. Wir folgen einlesen.

## Kernfluss (Nutzer-Skizze, angepasst an den Ist-Stand)

```
handler(cmd):                       # cmd.id = Envelope-ID; cmd.roh = volle Ingested-Nutzlast (Ist-Stand)
  buchung = aggregat.obfuskiere(cmd.roh, seed, obfuskator)   # seed-basiert via Port
  speicher.speichere(cmd.id, buchung)   # obfuskierte Buchung in NEUE Tabelle `obfuskiert`
  senke.publiziere(cmd.id)              # nur die ID publishen (Zeiger, nicht Nutzlast)
```

- **Ein `Ingested` = eine Zeile** (einlesen fächert pro Zeile eine Message) → eine Buchung
  je Message.
- **ID = Envelope-ID** der eingehenden `OutboxMessage` (opak, kein Leak) — Platzhalter, bis
  [0017](../issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)
  einlesen-vergebene Aggregat-IDs liefert.
- Die `obfuskiert`-Tabelle ist die Wahrheit; die `Obfuskiert`-Message trägt nur `{ id }`.

## Seeded-Obfuskierung (ersetzt First-Seen)

- **`Obfuskator`-Port** (io-erfüllt, „Random-Lib hinter Interface"): erzeugt aus
  **(Seed, Klartext)** deterministisch ein Token. Gleicher Seed + gleiche Eingabe → gleiches
  Token, **zustandslos** (kein First-Seen-Zähler, **keine** Klartext↔Token-Mapping-Tabelle).
  Reproduzierbarkeit von außen: mit Seed + Eingang lässt sich dasselbe Token nachrechnen.
- **Seed**: injiziert (Config-Seam, io-erfüllt; Fake in Specs) **und im Aggregat persistiert**
  (fließt in die `obfuskiert`-Zeile) → laufstabil, solange die Config denselben Seed liefert.
- **Was tokenisiert wird:** Auftraggeber- **und** Gegenkonto (`KONTO_…`), Personen-Name
  (`PERSON_…`). **Gläubiger-ID** bleibt Klartext-Anker. **Freitext** (Zweck/Buchungstext)
  läuft durch `ObfuskierterText` (Scrub minimal, vertagt) — **nicht** seed-tokenisiert.
- **Fail-safe Firma/Person** (Domänen-Business, CONTEXT.md 113–116): Gläubiger-ID vorhanden
  → Firma (Name Klartext als `ObfuskierterText`, Anker behalten); fehlt sie, Name vorhanden
  → Person (Name → `PERSON`-Token); weder noch → `Ohne`.
- **Pass-Through verbatim:** amount/currency/direction/Datumsfelder reicht der Slice
  unverändert durch (keine Re-Decodierung); sie ruhen mit im obfuskierten Aggregat.

## Zielstruktur (einlesen-gespiegelt)

`obfuskierung_domain`: `value_objects/` (`non_empty_string`, `seed`, `konto_token`,
`pseudonym`, `obfuskierter_text`, `gläubiger_id`, `empfänger`), `aggregate/`
(`ObfuskierungsAggregat`: hält Seed, entscheidet Fail-safe, ruft `Obfuskator`, baut
`ObfuskierteBuchung` inkl. Seed + Pass-Through), `ports/` (`Obfuskator`,
`ObfuskiertSpeicher`, `EreignisSenke`), `commands/` (`ObfuskiereBuchung` mit `id` + `roh`),
`handler/` (`ObfuskierHandler`, Seed als Konstruktor-Wert injiziert), Output
`ObfuskierteBuchung`.

`obfuskierung_adapter`: `dto/` (`EingelesenRecord` inbound serde + ACL `to_eingang` →
`MissingMandatoryField`; `ObfuskierteBuchung` outbound + Token-DTOs `KontoId`/`Pseudonym`/
`ObfuskierterText`/`Empfänger` + Pass-Through, **kein Serialize** + Guard), `commands/`
(`ObfuskiertMessage` pub(crate) serde, `OutboxCommand` „Obfuskiert", trägt nur `id`),
`abstractions/` (`ObfuskatorSeam`, `SeedSeam`, `ObfuskiertSpeicherSeam` — io-erfüllt),
`bridges/` (Consume via `TypedOutboxHandler`; Port-Bridges Seam→Domänen-Port; Publisher via
`OutboxPublisher`), `testapi/` (`ObfuskierungTestApi` + Fakes: `FakeObfuskator`
deterministisch, `FakeSeed`, `FakeObfuskiertSpeicher`, `FakeOutboxPublisher`).

`obfuskierung_specs`: Black-Box über die TestApi; projizierte `ObfuskierteBuchung` (aus dem
`obfuskiert`-Fake per ID gelesen) gegen erwartete DTO (ein `assert_eq!`); Fail-safe
Firma/Person; Seed-Determinismus (gleicher Seed+Eingang → gleiches Token, über Läufe
stabil); publizierte ID; Redaktion (Dead-Letter-Grund nie roh).

## Abweichungen vom Issue (nachzuziehen)

1. **Seeded-Obfuskierung statt First-Seen** `KONTO_x`/`PERSON_x` (Issue-AK 1) — Token via
   `Obfuskator`-Port + persistierter Seed; **keine** Klartext↔Token-Mapping-Tabelle.
   → berührt **ADR-0006** (Stabilität ruht auf Seed, nicht auf persistenter Mapping-Tabelle).
2. **„Token-Tabelle" → `obfuskiert`-Tabelle** des ganzen obfuskierten Aggregats; Message
   trägt nur die **ID** (nicht die Nutzlast).
3. Guard auf **Output-Token-DTO** (`ObfuskierteBuchung`), nicht auf `Iban` (Nutzer-Entscheid).
4. Alte freie `Obfuskierer`-API + 15 Specs **komplett gelöscht** (Nutzer-Entscheid).

## Aufgaben (Reihenfolge)

1. Branch — **erledigt** (`feat/obfuskieren-slice-pipeline-consumer`).
2. Zukunfts-Issue 0017 — **erledigt**.
3. Alten Slice-Code der drei Crates entfernen.
4. Specs schreiben (Black-Box) — kompiliert noch nicht.
5. Domäne + Adapter compiler-driven bauen, bis Specs grün.
6. Cargo.tomls (adapter: shared_kernel/application_kernel/serde/serde_json).
7. `cargo build`/`test`/`clippy --all-targets`/`fmt --check` + `cargo test -p architecture` grün.
8. Issue 0016 + ADR-0006 an die Abweichungen anpassen; Abnahme abhaken.

## Verifikation

Abnahmekriterien Issue 0016 (soweit nicht ersetzt); insbesondere Fail-safe getestet,
Seed-Determinismus/Reproduzierbarkeit, Guard auf Egress-DTO, architecture-Test grün, keine
`adapter→adapter`-Kante.
