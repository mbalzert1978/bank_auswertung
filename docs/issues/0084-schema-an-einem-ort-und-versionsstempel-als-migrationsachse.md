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

- [ ] Eine frisch angelegte Datenbank trägt die erwartete Schema-Version, und ein Lauf gegen sie geht durch.
- [ ] Ein zweiter Lauf gegen dieselbe Datei legt nichts neu an und läuft ebenfalls durch.
- [ ] Eine Datei mit abweichender Schema-Version lässt den Lauf terminal und redigiert scheitern — die Meldung nennt keine Rohdaten.
- [ ] Der produktive Öffnungspfad ist der einzige Weg, auf dem eine Datenbank ihr Schema bekommt; die Store-eigene Anlage ist nur noch Test-Weg (oder entfällt).
- [ ] Die sechs Gates sind grün (`/cargo-six-gates`).

## Blocked by

- ~~[0083](0083-lauf-anstoss-binary-reiht-den-ausloeser-ein-und-dreht-die-broker-loop.md)~~ —
  **aufgelöst**: der produktive Öffnungspfad (`io::run`) steht seit 0083.

> **Abgrenzung:** die **Entdopplung** wiederholter Importe ist **nicht** Teil dieses Tickets,
> sondern [0088](0088-zeilen-schluessel-lauf-eigene-fenster-und-vorgemerktes-ueberspringen.md) —
> sie ist die erste echte Migration auf dem Anker, den dieses Ticket legt.
