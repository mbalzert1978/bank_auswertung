---
id: "0084"
title: "Schema an einem Ort und Versionsstempel als Migrationsachse"
status: open
---

# Schema an einem Ort und Versionsstempel als Migrationsachse

**Typ:** AFK

> GitHub-Issue: [#101](https://github.com/mbalzert1978/bank_auswertung/issues/101)

## What to build

Heute legt **jeder** Ergebnis-Store sein eigenes Schema beim Konstruieren an. Für Tests ist
das bequem, für eine produktiv geöffnete Datei ist es eine Lücke: niemand besitzt das Schema
als Ganzes, und jede vorgefundene Datenbank steht ununterscheidbar auf Version 0. Eine
spätere Migration — etwa die im `io`-Kalenderhelfer bereits notierte Umstellung der
Tag-Stempel-Achse — hätte keinen Anker, an dem sie ansetzen könnte.

Der produktive Öffnungspfad legt das Schema an **einer** Stelle an und stempelt die Datei mit
der Schema-Version des Programms. Trifft er eine Datei mit abweichender Version an,
verweigert er den Dienst **terminal und redigiert**, statt auf einem fremden Schema
weiterzuschreiben. Die Store-eigene Anlage bleibt höchstens der Test-Weg und ist nicht mehr
der Weg, auf dem eine echte Datenbank entsteht.

## Acceptance criteria

- [x] Eine frisch angelegte Datenbank trägt die erwartete Schema-Version, und ein Lauf gegen sie geht durch.
  — `run::tests::eine_frische_datenbank_traegt_die_schema_version_und_der_lauf_geht_durch`,
  `schema::tests::eine_frische_datei_bekommt_alle_tabellen_und_die_schema_version`
- [x] Ein zweiter Lauf gegen dieselbe Datei legt nichts neu an und läuft ebenfalls durch.
  — `run::tests::ein_zweiter_lauf_gegen_dieselbe_datei_legt_nichts_neu_an_und_laeuft_durch`,
  `schema::tests::eine_passend_gestempelte_datei_wird_ohne_neuanlage_benutzt` (beide über SQLites
  `PRAGMA schema_version`, das mit jeder Schema-Änderung wächst)
- [x] Eine Datei mit abweichender Schema-Version lässt den Lauf terminal und redigiert scheitern — die Meldung nennt keine Rohdaten.
  — `run::tests::eine_datei_mit_abweichender_schema_version_laesst_den_lauf_terminal_und_redigiert_scheitern`
  (Alt-Datei mit Rohwerten; Meldung ohne Rohwert, Pfad und Dateinamen; Datei unangetastet),
  `schema::tests::eine_datei_mit_fremder_version_wird_abgewiesen_und_bleibt_unangetastet`,
  `schema::tests::eine_ungestempelte_datei_mit_tabellen_gilt_als_version_null_und_wird_abgewiesen`
- [x] Der produktive Öffnungspfad ist der einzige Weg, auf dem eine Datenbank ihr Schema bekommt; die Store-eigene Anlage ist nur noch Test-Weg (oder entfällt).
  — `schema::tests::ein_store_legt_kein_eigenes_schema_an`; `SliceStore::new` ist DDL-frei, die
  Store-eigene Anlage heißt `SliceStore::with_own_tables` und ist `#[cfg(test)]`
  (`crates/io/src/sqlite/slice_store.rs`)
- [x] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- ~~[0083](0083-lauf-anstoss-binary-reiht-den-ausloeser-ein-und-dreht-die-broker-loop.md)~~ —
  **aufgelöst**: der produktive Öffnungspfad (`io::run`) steht seit 0083.

> **Abgrenzung:** die **Entdopplung** wiederholter Importe ist **nicht** Teil dieses Tickets,
> sondern [0088](0088-zeilen-schluessel-lauf-eigene-fenster-und-vorgemerktes-ueberspringen.md) —
> sie ist die erste echte Migration auf dem Anker, den dieses Ticket legt.

## Wie es gebaut wurde

- **Ein Schema-Besitzer: `io::schema`** ([`crates/io/src/schema.rs`](../../crates/io/src/schema.rs)).
  `schema::open(&AppConfig)` ist der produktive Öffnungspfad, `io::run` ruft ihn. Er kennt **alle**
  Tabellen — die Outbox (`sqlite::OUTBOX_TABLE`, jetzt auch eine `TableSpec`) und die
  `StoreSpec::tables()` der vier Stores — und legt sie bei einer frischen Datei in **einer**
  Transaktion an, samt Stempel. Die DDL bleibt beim jeweiligen Store authored (ADR-0015); nur das
  Anlegen ist an eine Stelle gewandert. Das Modul liegt bewusst neben `sqlite`, nicht darin: das
  Rückgrat bleibt slice-blind.
- **Stempel = `PRAGMA user_version`**, `SCHEMA_VERSION = 1`. Eingebaut, transaktional, keine eigene
  Tabelle.
- **Drei Fälle, eine Weiche (`schema::ensure`):** keine Version und kein einziges Schema-Objekt →
  frisch, anlegen + stempeln; Version = `SCHEMA_VERSION` → benutzen, nichts anlegen; alles andere →
  `SchemaError::VersionMismatch`, im Lauf `RunError::SchemaVersion` (Exit 1). Die Meldung trägt nur
  die beiden Zahlen, nie Pfad, Dateiname oder Zeileninhalt. Geprüft wird **vor** dem ersten
  Schreiben: eine abgewiesene Datei bleibt byte-gleich.
- **Version 0 mit Tabellen ist „abweichend", nicht „frisch".** Eine solche Datei stammt aus der Zeit
  vor dem Stempel. Ob ihr Schema dem heutigen gleicht, weiß niemand — und laut CLAUDE.md sind die
  Zahlen eines wiederholten Laufs ohnehin falsch. Sie still zu stempeln hieße, genau das Raten
  fortzusetzen, das der Stempel beenden soll. Wer eine alte Datei hat, fängt mit einer frischen an.
- **`ensure_columns` ist entfallen** (samt `AUDIT_COLUMNS` und seinem Regressionstest). Es rüstete
  Audit-Spalten auf Alt-Dateien still nach — eine zweite, implizite Schema-Evolution neben der
  Version. Da Alt-Dateien jetzt abgewiesen werden, war es im produktiven Pfad tot.
- **Stores legen nichts mehr an.** `SliceStore::new(outbox)` ist DDL-frei und unfehlbar (vorher
  `without_schema`). Fehlt eine Tabelle doch, failt die Ablage wie bisher im Preflight terminal und
  redigiert. Der Test-Weg für isolierte Store-Tests heißt `SliceStore::with_own_tables` und ist
  `#[cfg(test)]`; ebenso `SqliteOutbox::open_in_memory` (nur Outbox-Tabelle). Für die externe
  Integrationsprobe gibt es `schema::open_in_memory()` — derselbe Besitzer, ohne Datei.
- **Anker für 0088 (Migration 1 → 2):** in `schema::ensure` ein Arm
  `Found::Version(1) => migrate_1_to_2(conn)`, dazu `SCHEMA_VERSION = 2` und die neue DDL in den
  Stores (eine frische Datei bekommt direkt Version 2). Mehr Maschinerie gibt es bewusst nicht.
