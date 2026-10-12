---
id: "0085"
title: "Poison-Message: Versuchszählung und Dead-Letter statt endloser Neuzustellung"
status: closed
---

# Poison-Message: Versuchszählung und Dead-Letter statt endloser Neuzustellung

**Typ:** AFK

> GitHub-Issue: [#102](https://github.com/mbalzert1978/bank_auswertung/issues/102)

## What to build

Eine Nachricht, deren Handler **terminal** scheitert, bleibt heute in der SQLite-Outbox
**offen** — jeder weitere Lauf stellt sie erneut zu und scheitert erneut. Dasselbe gilt für
einen wiederholt scheiternden Commit. Die Outbox-Mechanik in `io` benennt diese Lücke im
eigenen Doc-Kommentar selbst als „noch nicht gebaut": `OutboxError` kennt zwar bereits
`Retry`/`Warning`/`Critical`, aber niemand zählt Versuche und niemand nimmt eine Nachricht
je aus der Zustellung. Eine vergiftete Nachricht hält die Kette damit dauerhaft auf.

Die Outbox zählt Zustellversuche mit. Ein **terminaler** Fehlschlag (`Warning`/`Critical`)
nimmt die Nachricht **sofort** aus der Zustellung; ein **transienter** (`Retry`) erst,
nachdem eine konfigurierte Höchstzahl an Versuchen erschöpft ist. Aus der Zustellung
genommene Nachrichten bleiben mit ihrem **redigierten** Grund abfragbar, sodass ein Lauf sie
am Ende benennen kann, statt sie stillschweigend zu verlieren.

Das beantwortet die in
[ADR-0012](../adr/0012-durchweg-terminale-pipeline-und-uniformes-id-handoff.md) ausdrücklich
vertagte Frage, **wie `io` einen terminalen Fehlschlag surfacet**. Am Slice ändert sich
nichts: er gibt weiterhin nur seinen nackten, redigierten Domänen-Fehler zurück und kennt
weder Retry noch Dead-Letter.

## Acceptance criteria

- [x] Ein terminal scheiternder Handler nimmt seine Nachricht beim ersten Fehlschlag aus der Zustellung; ein Folgelauf stellt sie nicht erneut zu.
- [x] Ein transient scheiternder Commit wird bis zur konfigurierten Höchstzahl erneut zugestellt und danach aus der Zustellung genommen.
- [x] Die aus der Zustellung genommenen Nachrichten sind mit ihrem Grund abfragbar; ein Test nagelt fest, dass der Grund keinen Rohwert und keinen Freitext trägt.
- [x] Eine gesunde Nachricht hinter einer vergifteten wird weiterhin zugestellt — die Kette bleibt nicht stehen.
- [x] Der öffentliche Vertrag Richtung Slice bleibt unverändert (kein Slice lernt Retry oder Dead-Letter kennen).
- [x] Die sechs Gates sind grün (`/cargo-six-gates`).

## Wie es gebaut wurde

Kanonisch festgehalten in
[ADR-0017](../adr/0017-dead-letter-als-status-spalte-der-outbox.md).

**Die Antwort auf ADR-0012s vertagte Frage:** `io` surfacet einen terminalen Fehlschlag, indem der
**Broker** die Nachricht **aussondert** — sie bleibt als Outbox-Zeile liegen, wird aber nie wieder
zugestellt, und ihr redigierter Grund ist abfragbar. Der Slice-Vertrag (`application_kernel`, jeder
`_domain`/`_adapter`) ist unverändert; kein Slice kennt Retry oder Dead-Letter.

- **Form: Status-Spalte in der Outbox** (Plan, Schritt 4c), keine eigene Tabelle. `status` ∈
  `open`/`processed`/`dead_letter`, dazu `dead_letter_reason`. Zwei Tabellen-`CHECK`s machen die
  illegalen Kombinationen unkonstruierbar (`processed` ⇔ Ack-Stempel, `dead_letter` ⇔ Grund).
  `attempt` zählt jetzt jeden Versuch, auch den gelungenen.
- **Entscheidung im Broker** (`crates/io/src/sqlite/delivery.rs`, `SqliteOutbox::deliver_open`):
  nach jedem Versuch verbucht er den Ausgang. Die Regel ist die reine Funktion `disposition`:
  `Warning` (Slice) und `Critical` (Hülle) → sofort ausgesondert; `Retry` (Commit) → offen, bis
  `MAX_DELIVERY_ATTEMPTS` Versuche verbraucht sind. Gemeldet wird aus der getroffenen Zeile
  (`UPDATE … WHERE status = 'open' RETURNING …`): `Processed`, `Redelivery`, `DeadLettered`,
  `SettledElsewhere` (Zeile war schon erledigt, z. B. nach „bereits verarbeitet") oder `Unrecorded`
  (das Verbuchen scheiterte; die schon verbuchten Ausgänge bleiben, der Broker stellt danach nichts
  mehr zu).
- **Höchstzahl = Konstante `MAX_DELIVERY_ATTEMPTS = 3`**, keine Config-Datei (YAGNI): kein
  Fachparameter, sondern Broker-Mechanik. Ein Commit scheitert an der Datenbank selbst; hält das drei
  Versuche an, ist es nicht mehr vorübergehend.
- **Abfrage:** `SqliteOutbox::dead_letters()` → `DeadLetter` (Dispatch-Key, Versuche, Grund). Der Grund
  ist die `Display` des `OutboxError` — nie `partition_key` oder `content`.
- **Lauf:** beide Phasen laufen trotz Aussonderung zu Ende (die gesunden Nachrichten fließen weiter,
  die Analyse findet statt). Ein Lauf, der **selbst** etwas aussondert, endet mit
  `RunError::DeadLettered` → **Exit 1** und nennt es. Ein **späterer** Lauf wird nicht aufgehalten: die
  ausgesonderte Nachricht ist nicht offen, also auch kein `RunError::PendingImport` — das greift nur
  noch bei einem wirklich offenen Auslöser (abgestürzter Lauf). Die Schlussbilanz zählt frühere
  Aussonderungen mit („früher ausgesondert: n").
- **Fehlende Buchungen bleiben sichtbar (Review-Runde 1):** ein ausgesondertes `Ingested` fehlt für
  immer in `obfuskiert`. Jeder Lauf zählt darum am Ende die ausgesonderten `Ingested` der **Datei** →
  `RunReport::incomplete` (öffentlich, für den Bericht 0012) und „Analyse unvollständig: n Buchung(en)
  ausgesondert" im Protokoll — bei Exit 0. Ein ausgesonderter `ReadTransactions` zählt nicht (Batch
  atomar; ein erneuter Import füllt den ganzen Auszug).
- **Schema v4** (`crates/io/src/schema.rs`): Sprosse 3 → 4 der Migrations-Leiter aus 0009 baut die
  Outbox mit **eingefrorener** Ziel-DDL um und übernimmt den Bestand (`processed_on_utc` gesetzt →
  `processed`, sonst `open`; `rowid` bleibt). Der Version-1-Neubau baut die Outbox ebenso um.
- **Bewusst nicht gebaut:** kein Redrive-Werkzeug (YAGNI) — siehe ADR-0017.

**Belegende Tests** (crate-intern in `io`, echtes SQLite):

| AK | Test |
|---|---|
| terminal → sofort raus, kein Folge-Zustellen | `sqlite::delivery::tests::ein_terminal_scheiternder_handler_wird_beim_ersten_fehlschlag_ausgesondert_und_nie_wieder_zugestellt`, `…eine_hoffnungslose_huelle_wird_als_critical_sofort_ausgesondert` |
| transient bis Höchstzahl, dann raus | `…ein_transient_scheiternder_commit_wird_bis_zur_hoechstzahl_zugestellt_und_dann_ausgesondert` (+ `…ein_commit_der_beim_zweiten_versuch_gelingt_…`) |
| abfragbar, Grund ohne Rohwert/Freitext | `…ausgesonderte_nachrichten_sind_mit_grund_abfragbar_und_der_grund_traegt_keinen_rohwert_und_keinen_freitext`, `run::tests::der_aussonderungs_grund_eines_echten_laufs_traegt_keinen_rohwert_und_keinen_freitext` |
| gesunde Nachricht hinter vergifteter | `…eine_gesunde_nachricht_hinter_einer_vergifteten_wird_weiter_zugestellt`, `run::tests::ein_ausgesonderter_ausloeser_haelt_den_naechsten_lauf_nicht_auf`, `run::tests::eine_in_einem_frueheren_lauf_ausgesonderte_buchung_macht_jeden_spaeteren_lauf_unvollstaendig` |
| Slice-Vertrag unverändert | Diff berührt nur `crates/io`, `crates/lauf` und Doku — kein `_domain`/`_adapter`/`_specs`, kein `application_kernel` |
| Migration | `schema::tests::eine_version_3_datei_mit_bestand_bekommt_die_status_spalte_und_behaelt_jede_nachricht` (nicht aufsteigende `rowid`s) |
| Regel, Randfälle | `…die_regel_sondert_terminal_sofort_und_transient_erst_beim_letzten_versuch_aus` (ohne SQLite), `…eine_anderweitig_erledigte_nachricht_wird_weder_gezaehlt_noch_ausgesondert`, `…ein_scheiterndes_verbuchen_verwirft_keinen_ausgang_und_haelt_die_zustellung_an` |

## Blocked by

- None — can start immediately.
