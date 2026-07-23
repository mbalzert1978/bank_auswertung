# SQL — Queries & Performance

Beispiele im Postgres-Dialekt. Hinweis: die reale io-Persistenz ist **SQLite**
([ADR-0014](../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)), das keine
Materialized Views kennt — dort eine Ergebnistabelle mit periodischem Neuaufbau als
Äquivalent.

## Nur benötigte Spalten selektieren — nie `SELECT *`

In Produktion nur die tatsächlich benötigten Spalten selektieren, niemals `SELECT *`.

Do:

```sql
SELECT id, betrag, booking_day
FROM   obfuskiert
WHERE  booking_day >= DATE '2026-01-01';
```

Don't:

```sql
SELECT *                       -- Overfetch: liest ungenutzte Spalten (IO), bricht still bei Schema-Änderung
FROM   obfuskiert
WHERE  booking_day >= DATE '2026-01-01';
```

## Materialized Views für teure, häufig gelesene Aggregationen

Teure Aggregationen, die häufig gelesen werden, als Materialized View vorberechnen.

Do:

```sql
CREATE MATERIALIZED VIEW monats_cashflows AS
SELECT date_trunc('month', booking_day) AS monat,
       SUM(betrag)                       AS summe
FROM   obfuskiert
GROUP  BY 1;
-- periodisch aktualisieren: REFRESH MATERIALIZED VIEW monats_cashflows;
```

Don't:

```sql
-- dieselbe teure Aggregation bei jedem Read neu: voller Scan + GROUP BY pro Request
SELECT date_trunc('month', booking_day) AS monat, SUM(betrag)
FROM   obfuskiert
GROUP  BY 1;
```

## Fremdschlüssel-Spalten indizieren, Query-Pläne analysieren

Fremdschlüssel-Spalten automatisch indizieren und die Query-Pläne langsamer Abfragen
analysieren.

Do:

```sql
CREATE TABLE auswertungen (
    id         BIGINT PRIMARY KEY,
    fenster_id BIGINT NOT NULL REFERENCES auswertungsfenster (id)
);
CREATE INDEX idx_auswertungen_fenster_id ON auswertungen (fenster_id);  -- FK indiziert
EXPLAIN ANALYZE SELECT id FROM auswertungen WHERE fenster_id = 42;      -- Plan bestätigt Index-Scan
```

Don't:

```sql
CREATE TABLE auswertungen (
    id         BIGINT PRIMARY KEY,
    fenster_id BIGINT NOT NULL REFERENCES auswertungsfenster (id)
    -- kein Index auf fenster_id: jeder JOIN/Filter erzwingt einen Full-Table-Scan
);
```
