# Modularer Monolith aus Feature-Slices — die Domäne ist blind, der Shared-Kernel ist dünn

> **Status: in Teilen superseded — durch [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)
> und [ADR-0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md).**
> Der Textkörper unten ist **Historie** (Entscheidungsstand zum ADR-Zeitpunkt) und
> bleibt unverändert. Was heute gilt und was nicht mehr, steht **kanonisch** im
> Abschnitt „Was dieses ADR supersedet" von [ADR-0008](0008-event-getriebene-pipeline-und-outbox.md)
> (Pipeline) bzw. [ADR-0009](0009-kernel-split-und-idiomatische-outbox-vertraege.md)
> (der hier „dünne `shared_kernel`" wird zu **zwei** Kernel-Crates).

Wir schneiden die Software in **Feature-Slices** statt in eine geteilte Domäne
plus Adapter. Auslöser waren zwei Kopplungen, die der bisherige Aufbau
([ADR-0001](0001-hexagonale-architektur.md), [0002](0002-dto-grenze-und-anti-corruption-layer.md),
[0003](0003-rust-als-sprache.md)) nicht verhindert hat:

1. Der eingehende Port gab ein Domänenobjekt heraus (`Umsatzquelle::buchungen()
   -> Vec<Buchung>`) und koppelte jeden Aufrufer direkt an Domänen-Interna.
2. Ein **geteiltes** `domain`-Crate (`Buchung`, Value Objects, `Rolle`) hätte alle
   künftigen Features aneinandergekoppelt — genau die Kopplung, die die Trennung
   vermeiden soll.

Dieses ADR **verfeinert/supersedet** die Architektur-Aussagen von 0001–0003 (die
Datenschutz- und Modell-Prinzipien bleiben, ihre Verortung „in **der** einen
Domäne" nicht). Vorbild ist die interne C#-Referenz (`Downloader/`-Slice +
`Common/`-Shared-Kernel).

## Der Schnitt: Slice-Triaden + dünner Shared-Kernel

Jedes Feature ist eine **Triade** aus drei Crates:

- **`<slice>_domain`** — die Aggregate (rich domain: Verhalten in Methoden, nicht
  anämisch) und die **Ports nach außen**, die die Domäne „verlangt".
- **`<slice>_adapter`** — implementiert die Domänen-Ports (Bridges), **definiert
  die public Seams** (die Interfaces, die IO erfüllt), hält die public **DTOs**,
  die Projektionen und die public **TestApi** (Fluent-Builder) samt Fakes.
- **`<slice>_specs`** — nur Tests; hängt allein am `<slice>_adapter` und nutzt
  dessen public TestApi + DTOs. Black-Box strukturell erzwungen.

Der **Shared-Kernel** ist ein **dünnes, interface-only** Crate auf
Adapter-Niveau — nur Cross-Cutting-Verträge (`Handler`, `Command`,
`PublishCommand`, `Publisher`), keine Value Objects, keine Geschäftsdomäne. Er
läuft nicht auf der Domäne zusammen (sonst wären die Slices wieder gekoppelt) und
wird erst von Adaptern genutzt, sobald das Messaging kommt.

## Die blinde Domäne

`<slice>_domain` hat **keine internen Dependencies** — auch nicht auf den
Shared-Kernel. Sie kennt nur ihre eigenen Ports nach außen und „verlangt" alles
Nötige über diese; sie weiß nicht, dass ein Slice sie referenziert. Ihre
**öffentliche Fläche ist schmal**: `pub` sind nur die Ports und die
Aggregate-Roots (mit ihren Methoden, ohne `pub`-Felder). **Entities, die in einem
Aggregat sitzen, sind nicht `pub`**; alles Übrige ist crate-intern.

Die Domäne ist zudem **dependency-frei auch von externen Crates**: keine
Fehler-Bibliothek (`thiserror`) und **kein `serde`**. Sie gibt ihren Fehler als
schlichten Typ vor (`Display` + `std::error::Error`, nur über `std`); der Adapter
mappt/umschließt ihn (dort gern mit `thiserror`). Ohne `serde`-Dep kann ein
struktureller Identifikator in der Domäne gar kein `Serialize` ableiten — der
compilergeprüfte `assert_not_impl_any!`-Beleg lebt am Rand, auf dem Adapter-DTO.

## Naming: Entity/Aggregat in der Domäne, UBQL-DTO am Rand

- **Domäne** arbeitet mit Entities und Aggregaten und hängt das Suffix an:
  `IbanEntity`, `BuchungAggregat`. Die Einstufung (Entity vs. Aggregat vs. VO)
  folgt DDD.
- **Slices/Adapter** reden UBQL als **reine DTOs** mit dem schlichten
  Glossar-Namen — `Iban`, `Rolle`, `Buchung`, `Betrag` (**kein** `Dto`-Suffix).
- **Technik** bleibt englisch (`Handler`, `Publisher`, `TestApi`, `new`,
  `from_str`); Domänenvokabular deutsch ([CONTEXT.md](../../CONTEXT.md)).

## Datenfluss: IO → typisierter DTO → Adapter → Domäne

Ein **einziges `io`-Crate** liest roh (z. B. Dateizeilen als `str`), baut daraus
den vom Seam verlangten **DTO** über dessen selbstvalidierenden Konstruktor
(`Iban::from_str`/`new` — die Validierung lebt im DTO) und reicht ihn an den
Adapter-Port hinunter (`io` implementiert den Seam). Der Adapter mappt den DTO auf
die Domänen-Entity, treibt das Aggregat über dessen Methoden und projiziert das
Ergebnis zurück auf einen Ausgangs-DTO. So läuft IO nie über ein Domänen-Interface
(Vorbild `Downloader/Adapter/SftpPullerAdapter.cs`). Feld-Semantik (deutscher
Betrag `1.234,56` → `Betrag`) lebt im jeweiligen DTO-Konstruktor.

## Slice→Slice nur über public Ports

Slices greifen **nie** direkt aufeinander zu, sondern nur über die public Ports
des fremden `<slice>_adapter` — und mappen dessen DTOs auf ihre **eigenen**
Domänentypen (Vorbild `Downloader/Adapter/DownloadLedgerAdapter.cs`). Konsequenz
des dünnen Kernels: gemeinsame Begriffe wie `Rolle` werden je Slice modelliert und
an der Grenze über DTOs übersetzt, nicht geteilt.

## Obfuskierung als eigener Slice

Die Pseudonymisierung (IBAN→`KONTO_x`, Name→`PERSON_x`, später Freitext-Scrubbing)
wird ein eigener Slice mit public API `obfuskiere_*`. Er besitzt die
Klartext↔Token-Zuordnung als eigenen, getrennten Store (persistent + laufstabil,
[ADR-0006](0006-pseudonym-stabilitaet.md); Persistenz vertagt). Rohe
Identifikatoren gehen in diesen Slice hinein, verlassen aber nie die Maschine. Der
CSV-Slice konsumiert ihn über eine Bridge, **bevor** seine Domäne etwas sieht.

## Dependency-Richtung: erzwungen und getestet

Alle Kanten zeigen nach innen: `presentation → io`/`*_adapter`;
`io → *_adapter`; `<slice>_adapter → <slice>_domain`, `shared_kernel` und (für
Slice→Slice) fremde `*_adapter`; `<slice>_specs → <slice>_adapter`;
`*_domain`/`shared_kernel` ohne interne Deps. Externe Fremd-Crates (`csv`, `toml`,
`sqlx`, …) leben ausschließlich im `io`-Crate. Ein Top-Crate **`architecture`**
(außerhalb `crates/`, referenziert alle Crates, darf sich frei bedienen —
`cargo_metadata`) prüft diese Kanten als Test; der bestehende PreToolUse-Hook
prüft dieselbe Regel schon zur Edit-Zeit.

## Rehydrate-Invariante (Vorbehalt)

Sobald IO persistiert, muss die Domäne aus dem gespeicherten DTO **verlustfrei
rehydrierbar** sein (round-trip-vollständig). Erweist sich das mit der strikten
Kapselung (schmale Domänenfläche, Entities im Aggregat privat) als nicht
realisierbar, entfällt diese Beschränkung. Gestartet wird bewusst strikt.

## Konsequenzen

- **Das geteilte `domain`-Crate wird aufgelöst.** Seine Typen wandern in den
  Slice, der sie besitzt (`Buchung`/VOs → `csv_camt_v8`; `Iban`/`Rolle`-Register →
  `eigene_konten`; `KONTO_x`/`PERSON_x` → `obfuskierung`). Aktuell ungenutzte
  Typen (`KategorieAnfrage`, `Fingerabdruck`, der `strukturelle_identifikatoren`-
  Platzhalter) entfallen/werden vertagt, bis ihr Slice entsteht.
- **Die Datenschutz-Invariante bleibt, nur pro Slice re-homed:** strukturelle
  Identifikatoren (`Iban`/`Bic`) leiten kein `Serialize` ab (compilergeprüft via
  `assert_not_impl_any!`), im jeweiligen Slice, der sie berührt — nicht zentral.
  Die Leak-Garantie ruht weiter auf dem fehlenden `Serialize`, nicht auf der
  Abwesenheit von `serde`.
- **Vertagt (Zielbild, nicht in diesem Umbau):** das einzige `io`-Crate
  (`csv`/`toml`/`fs`/`sqlx` + Speicher — Speicher ist **kein** Slice, sondern IO),
  die `presentation`-Crate als Presenter (kein Slice; fragt den finalen Slice ab
  und bedient dessen public Port), die Messaging-Schicht (interne öffentliche API,
  Commands, Publisher, Outbox, SQLite-Broker) sowie die Analyse-/Kategorisierer-
  Slices.
- Ein weiteres Feature ist eine neue Triade, kein Eingriff in bestehende Slices.
