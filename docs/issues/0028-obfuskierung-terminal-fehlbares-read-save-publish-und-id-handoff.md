---
id: "0028"
title: "obfuskierung: terminal-fehlbares read/save/publish und ID-Handoff"
status: closed
---

# obfuskierung: terminal-fehlbares read/save/publish und ID-Handoff

## Kontext

Der `obfuskierung`-Slice weicht in zwei Punkten vom einheitlichen Pipeline-Muster ab,
das `einlesen` bereits umsetzt:

1. **Fehler-Modellierung.** Der von der Domäne verlangte Lese-Port liefert heute einen
   eigenen dreiwertigen Aufzähltyp (`Found`/`Absent`/`Malformed`) statt eines `Result`,
   und der Handler übersetzt diesen 1:1 in einen **Duplikat-Fehler** (`HandleError`) —
   ein überflüssiger `match`-Arm und zwei spiegelbildliche Fehlertypen. Beide Fehler-Äste
   sind ohnehin **terminal**; der `Malformed`-Ast ist im gedachten Fluss nicht erreichbar
   (die Präsenz der Pflichtfelder garantiert bereits `einlesen` beim Ablegen).
2. **ID-Handoff.** Die Ablage ist heute unfehlbar (`void`) und der Slice publiziert die
   **Quell-ID** der gelesenen Buchung als Zeiger. Richtig ist: die Ablage schreibt die
   obfuskierte Buchung in ihre **eigene** Zeile, deren **neue** ID die Persistenz vergibt,
   und **diese** neue ID reist als Zeiger weiter. So macht es `einlesen` schon
   (Ablage vergibt die Identität, genau sie wird publiziert).

Die Doku-Ebene (ADR + `CLAUDE.md`) für dieses Muster ist ein eigenes Issue; hier geht es
rein um den `obfuskierung`-Code.

## What to build

Den `obfuskierung`-Slice (Domäne + Adapter) auf das einheitliche Muster ziehen: **read,
save und publish sind je fehlbar und terminal**, und die publizierte ID ist die von der
Ablage neu vergebene ID.

- Der Lese-Port liefert die rohe Buchung als `Result` mit **einem** redigierten,
  terminalen Slice-Fehler (`SourceError`) statt des dreiwertigen Aufzähltyps; der bisherige
  Aufzähltyp und der spiegelbildliche Handler-Fehler entfallen. Der Fehler trägt **nie einen
  Rohwert** — nur einen statischen Feldnamen bzw. die opake ID; sein `Display` ist ein
  **terminaler Fehlergrund**, nicht als „Dead-Letter" bezeichnet.
- Der Ablage-Port ist fehlbar und gibt die **neu vergebene ID** der obfuskierten Buchung
  zurück; der Publish-Port ist fehlbar.
- Der Handler wird eine geradlinige `?`-Kette: lesen → obfuskieren → ablegen (neue ID) →
  neue ID publizieren. Kein Übersetzungs-`match` mehr.
- Die obfuskierte Buchung referenziert die Quell-Buchung **nur per ID** (Fremdschlüssel),
  nicht als publizierten Zeiger.
- Die öffentlichen Adapter-Seams (Ablage-Seam gibt ID + ist fehlbar, Publish-Seam fehlbar),
  die In-Memory-Fakes (vergeben IDs wie der `einlesen`-Fake), die TestApi-Rückgabe und die
  Blackbox-Specs ziehen mit; der Slice re-exportiert den neuen `SourceError` statt des
  gelöschten Duplikat-Fehlers.
- Docstrings im Zuge dessen **entrümpeln**: „Dead-Letter"-Formulierungen und ausufernde
  Überlegungen/Rationale raus (die Begründung lebt im ADR), nur noch Vertrag und Verhalten.
  Keine Verweise auf das gelöschte `examples/`.

Jeder Slice trägt seinen **eigenen** `SourceError` (Namensgleichheit mit anderen Slices ist
akzeptiert) — **kein** geteilter Fehler-Crate.

## Acceptance criteria

- [ ] Der Lese-Port liefert `Result<…, SourceError>` (die Buchung **nicht** geboxt); der
      dreiwertige Aufzähltyp ist entfernt.
- [ ] Der Duplikat-Handler-Fehler ist entfernt; der Handler propagiert Lese-, Ablage- und
      Publish-Fehler per `?`, ohne Übersetzungs-`match`.
- [ ] Ablage ist fehlbar und gibt die neu vergebene ID zurück; der Handler publiziert **diese**
      ID, nicht die Quell-ID.
- [ ] `SourceError` ist terminal und redigiert (kein Rohwert; nur Feldname/opake ID); keine
      „Dead-Letter"-Formulierung mehr im Slice.
- [ ] Docstrings entrümpelt (keine Rationale-Ausschweifungen, keine `examples/`-Verweise).
- [ ] `cargo build`, `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` und
      `cargo test -p architecture` sind grün.

## Blocked by

None - can start immediately.
