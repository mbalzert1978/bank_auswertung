---
id: "0081"
title: obfuskierung-Domaenenfehler in ObfuscationError umbenennen
status: closed
---

# obfuskierung-Domaenenfehler in ObfuscationError umbenennen

Löst eine Namensdrift zwischen den vier Slices **und** zwischen Regel und Code. **AFK.**

## What to build

`obfuskierung` nennt sein **slice-weites** Fehler-Enum `SourceError`, obwohl darin längst
nicht nur Lese-Fehler sitzen, sondern auch der Fehlschlag beim Ablegen. Die Schwester-Slices
benennen ihres nach dem Slice, nicht nach einem seiner I/O-Schritte (`fenstern`:
`WindowError`, `auswerten`: `EvaluationError`) — `obfuskierung` ist der Ausreißer. Erschwerend
kommt hinzu, dass das Do-Beispiel in der Testkultur-Regel bereits `ObfuscationError` als
öffentliche Adapter-Fläche zeigt: die Regel dokumentiert einen Namen, den der Code nicht hat.

Der Slice benennt das Enum auf der ganzen öffentlichen Adapter-Fläche in `ObfuscationError`
um und zieht Domäne, Ports, Port-Bridges, TestApi, io-Verdrahtung und die Black-Box-Specs
nach. Das gleichnamige, **fachlich unabhängige** `SourceError` des `einlesen`-Slice bleibt
unangetastet — dort ist der Name korrekt, weil er dort tatsächlich nur die Quelle betrifft.

Läuft **nach** [0078](0078-obfuskierung-lese-seam-fehlbar-machen.md), weil dieses zuvor eine
weitere Variante in dasselbe Enum einzieht; andernfalls kollidieren beide Tickets an
derselben Stelle.

## Acceptance criteria

- [ ] Der slice-weite Domänenfehler des `obfuskierung`-Slice heißt `ObfuscationError`; im
      Slice ist kein Rest des alten Namens mehr auffindbar.
- [ ] Das gleichnamige Enum des `einlesen`-Slice ist unverändert.
- [ ] Die öffentliche Adapter-Fläche exportiert den neuen Namen, und die Black-Box-Specs
      nutzen ihn — die Specs bleiben rein auf der öffentlichen Fläche.
- [ ] Das Do-Beispiel in der Testkultur-Regel stimmt mit der tatsächlichen öffentlichen
      Fläche überein.
- [ ] Alle sechs Gates grün (`/cargo-six-gates`).

## Blocked by

- [0078](0078-obfuskierung-lese-seam-fehlbar-machen.md) — zieht zuvor eine weitere Variante
  in dasselbe Enum ein.
