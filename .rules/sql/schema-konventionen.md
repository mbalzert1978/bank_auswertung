# SQL — Schema-Konventionen

Beispiele im Postgres-Dialekt. Hinweis: die reale io-Persistenz dieses Repos ist **SQLite**
([ADR-0014](../../docs/adr/0014-io-persistenz-rusqlite-synchron-kein-sqlx.md)) — einzelne
Features (`uuid_generate_v4`) sind Postgres-spezifisch und dort sinngemäß umzusetzen.

## snake_case und Plural-Tabellennamen

Für alle Tabellen- und Spaltennamen snake_case verwenden; Tabellennamen im Plural.

Do:

```sql
CREATE TABLE bank_accounts (
    id         BIGINT PRIMARY KEY,
    iban_token TEXT NOT NULL
);
```

Don't:

```sql
CREATE TABLE BankAccount (      -- CamelCase + Singular: uneinheitlich, Case-Folding-Fallen je nach DB
    ID        BIGINT PRIMARY KEY,
    IBANToken TEXT NOT NULL
);
```

## created_at und updated_at auf jeder Tabelle

Jede Tabelle bekommt `created_at`- und `updated_at`-Zeitstempel.

Do:

```sql
CREATE TABLE obfuskiert (
    id         BIGINT PRIMARY KEY,
    konto      TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Don't:

```sql
CREATE TABLE obfuskiert (
    id    BIGINT PRIMARY KEY,
    konto TEXT NOT NULL
    -- ohne Zeitstempel: Änderungszeitpunkt/Audit/Debugging nicht rekonstruierbar
);
```

## UUID- oder nanoid-Primärschlüssel in verteilten Systemen

In verteilten Systemen UUIDs (`uuid_generate_v4`) oder nanoid als Primärschlüssel verwenden.

Do:

```sql
CREATE TABLE ingested_lines (
    id     UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    betrag BIGINT NOT NULL
);
```

Don't:

```sql
CREATE TABLE ingested_lines (
    id     BIGSERIAL PRIMARY KEY,  -- knoten-lokal monoton: IDs kollidieren beim Merge mehrerer Knoten
    betrag BIGINT NOT NULL
);
```

## Check-Constraints für enum-artige Werte

Enum-artige Werte per Check-Constraint auf Datenbank-Ebene erzwingen.

Do:

```sql
CREATE TABLE gegenkonten (
    id    BIGINT PRIMARY KEY,
    rolle TEXT NOT NULL CHECK (rolle IN ('giro', 'haushalt', 'spar', 'vorlauf'))
);
```

Don't:

```sql
CREATE TABLE gegenkonten (
    id    BIGINT PRIMARY KEY,
    rolle TEXT NOT NULL   -- ohne CHECK: 'Giro', 'sparen' oder Tippfehler landen ungeprüft in der DB
);
```
