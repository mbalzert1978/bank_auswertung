---
id: "0030"
title: "ADR-0012: durchweg-terminale Pipeline und uniformes ID-Handoff"
status: open
---

# ADR-0012: durchweg-terminale Pipeline und uniformes ID-Handoff

## Kontext

Das über die Slices `einlesen`/`obfuskierung` etablierte Muster war bisher **nur in
Docstrings** verstreut beschrieben — teils falsch (das „Dead-Letter"/„io klassifiziert
Warning/Retry/Critical"-Framing beruhte auf einem Missverständnis von Beginn an) und mit
ausufernder Rationale, die nicht in Docstrings gehört. Die Slice-Issues (0028/0029)
entfernen diese Rationale aus dem Code; sie braucht einen **kanonischen Ort**.

Der zu dokumentierende Stand:

- Die Pipeline ist **durchweg terminal**: **kein Dead-Letter, kein Retry im Slice**.
  Scheitert ein Schritt (Quelle nicht lesbar, Ablage/Publish scheitert), ist das terminal.
  Der Slice gibt seinen **nackten, redigierten** terminalen Domänen-Fehler zurück; wie `io`
  (vertagt) einen terminalen Fehlschlag surfacet, ist dort offen.
- **Alle drei** I/O-Schritte jedes Slice — lesen, ablegen, publizieren — sind **fehlbar**.
- **Uniformer ID-Handoff:** jeder Slice liest die Vorgänger-Zeile per ID, baut sein Aggregat,
  transformiert, schreibt in seine **eigene** neue Tabelle (die Persistenz vergibt die ID),
  und publiziert **diese neue ID** als Zeiger auf den nächsten Schritt. `einlesen` ist der
  Kopf (Quelle ist eine Datei hinter dem Seam, per Auslöser getriggert statt per
  Vorgänger-ID).
- **Aggregate referenzieren Fremd-Entitäten nur per ID** (Fremdschlüssel), nie durch
  eingebettete fremde Objekte.

Dieses Muster **verfeinert/supersediert** ADR-0008 (event-getriebene Pipeline/Outbox) und
ADR-0011 (Slice-reine Pipeline, Mechanik in `io`).

## What to build

Einen neuen **ADR-0012**, der dieses Muster als Entscheidung festhält, plus die Angleichung
von `CLAUDE.md`.

- ADR-0012 nach dem im Repo etablierten ADR-Format: benennt die Entscheidung
  (durchweg-terminal, kein Dead-Letter; alle I/O-Schritte fehlbar; DB-vergebene ID als
  publizierter Zeiger; Aggregate referenzieren Fremd-Entitäten nur per ID), mit Begründung
  und Konsequenzen. Nimmt die aus den Slice-Docstrings entfernte Rationale auf.
- Die **superseded/verfeinert-Beziehung** zu ADR-0008 und ADR-0011 nach dem **bekannten
  Muster** setzen: der Banner oben auf 0008/0011 mit kanonischem Verweis auf 0012, so wie
  die bestehenden Alt-ADRs ihre Superseded-Banner tragen.
- `CLAUDE.md` an den korrigierten Stand angleichen: „alles terminal, kein Dead-Letter";
  fehlbares read/save/publish; der `obfuskierung`-ID-Handoff; das gelöschte `examples/` wird
  nicht mehr referenziert.

## Acceptance criteria

- [ ] ADR-0012 existiert, im Repo-ADR-Format, und beschreibt: durchweg-terminale Pipeline
      (kein Dead-Letter/Retry), fehlbares read/save/publish, DB-vergebene ID als Zeiger,
      Aggregate referenzieren Fremd-Entitäten nur per ID.
- [ ] ADR-0008 und ADR-0011 tragen den Superseded/verfeinert-Banner mit kanonischem Verweis
      auf ADR-0012, nach dem im Repo bestehenden Muster.
- [ ] `CLAUDE.md` ist konsistent zum ADR (terminal statt Dead-Letter; fehlbares
      read/save/publish; `obfuskierung`-ID-Handoff) und referenziert `examples/` nicht mehr.
- [ ] Interne Verweise/Links stimmen (ADR-Index bzw. Querverweise auf 0012 vorhanden).

## Blocked by

None - can start immediately.
