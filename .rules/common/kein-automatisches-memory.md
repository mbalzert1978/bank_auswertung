# Kein automatisches persistentes Memory

## Nichts selbsttätig ins Memory schreiben — erst den Nutzer fragen

Das sitzungsübergreifende Memory-System (das projekteigene `memory/`-Verzeichnis inkl.
`MEMORY.md`) ist bewusst geleert und bleibt es. Taucht etwas über die Sitzung hinaus
Festzuhaltendes auf, **zuerst den Nutzer** nach dem Ablageort fragen (diese `.rules/`,
[CLAUDE.md](../../CLAUDE.md), die [ADRs](../../docs/adr/), [CONTEXT.md](../../CONTEXT.md) oder
sonstige Repo-Doku) und dort ablegen.

Do:

```text
Nutzer: "Merk dir, dass wir Tickets doppelt anlegen."
→ Rückfrage: "Soll das als Regel nach .rules/common/tickets.md?" → dort ablegen.
```

Don't:

```text
→ still eine neue Datei in memory/ anlegen und MEMORY.md ergänzen  // abgeschaltet
```
