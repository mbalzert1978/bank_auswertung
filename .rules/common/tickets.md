# Tickets / Issues

## Tickets doppelt anlegen — GitHub UND `docs/issues/`

Jedes Ticket lebt an zwei synchron gehaltenen Orten: GitHub via `gh` (Tracker) und eine
lokale, versionierte Kopie unter [docs/issues/](../../docs/issues/) (fortlaufende Nummer,
etabliertes Frontmatter-Format wie 0050–0059) als im Git nachvollziehbare kanonische Kopie.
Nie nur eine der beiden Seiten.

Do:

```bash
gh issue create --title "…" --body-file ticket.md
cp ticket.md docs/issues/0060-slug.md   # GitHub-Issue-Nummer im File referenzieren
```

Don't:

```bash
gh issue create --title "…" --body "…"  # nur GitHub, keine versionierte Kopie
```
