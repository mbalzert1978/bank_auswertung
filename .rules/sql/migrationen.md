# SQL — Migrationen

## Migrationen auf einer Produktionsdaten-Kopie testen

Migrationen vor dem Produktionslauf auf einer Kopie der Produktionsdaten testen, nie
ungetestet direkt in Produktion fahren.

Do:

```sql
-- Auf der Kopie (z. B. via `pg_dump prod | psql staging_copy`) fahren und verifizieren:
BEGIN;
ALTER TABLE obfuskiert ADD COLUMN seed_marker TEXT NOT NULL DEFAULT '';
-- Daten-/Constraint-Prüfungen auf realem Volumen ...
ROLLBACK;  -- erst auf der Kopie prüfen, dann bewusst in prod ausrollen
```

Don't:

```sql
-- ungetestet direkt gegen die Produktionsdatenbank:
ALTER TABLE obfuskiert ADD COLUMN seed_marker TEXT NOT NULL;  -- NOT NULL ohne DEFAULT scheitert an vorhandenen Zeilen / lockt die Tabelle
```
