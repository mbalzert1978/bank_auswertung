# Nicht verhandelbare Invarianten

Der Grund, warum das Projekt existiert. **Nicht abschwächen.** Maßgeblich:
[ADR-0002](../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md),
[ADR-0004](../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md),
[ADR-0006](../../docs/adr/0006-pseudonym-stabilitaet.md),
[ADR-0010](../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md).

## Kein `Serialize` auf Egress-DTOs

Der compiler-erzwungene „kein `Serialize`"-Guard sitzt auf den Egress-DTOs
(Kategorisierer-Anfrage + Report-Ausgabe). Einen rohen Identifikator in etwas Ausgehendes
einzubetten ist ein **Compile-Fehler**, kein Review-Versäumnis.

Do:

```rust
pub struct ObfuscatedBooking { konto: Pseudonym, betrag: Betrag }
assert_not_impl_any!(ObfuscatedBooking: serde::Serialize); // Leak = Compile-Fehler
```

Don't:

```rust
#[derive(Serialize)]
pub struct ObfuscatedBooking { konto_iban: Iban, betrag: Betrag } // rohe IBAN verlässt die Maschine
```

## Strukturelle Identifikatoren verlassen die Maschine nie

Eigene/Gegen-IBAN, Kontonummer, BIC sind roh nur **lokal** bis `obfuskieren` erlaubt; ab da
fließen nur Tokens. Ausnahme: die öffentliche `Gläubiger-ID` als Analyse-Anker.

Do:

```rust
// auswerten sieht nur Tokens
let gegenkonto: Pseudonym = booking.gegenkonto_token();
```

Don't:

```rust
// nachgelagert eine rohe IBAN weiterreichen
let gegenkonto: Iban = booking.gegen_iban(); // ab obfuskieren verboten
```

## Freitext nur über `ObfuskierterText`

Empfänger/Verwendungszweck müssen durch das eine Value Object `ObfuskierterText`, dessen
Konstruktor best-effort scrubbt — **heuristisch, nicht beweisbar vollständig** (ehrlich so
benennen, keine Anonymisierung behaupten).

Do:

```rust
let zweck = ObfuskierterText::new(rohtext); // scrubbt beim Konstruieren
```

Don't:

```rust
let zweck: String = rohtext; // roher Freitext wandert ungefiltert weiter
```

## Fail-safe: im Zweifel `Person`

Ist Firma vs. Privatperson unklar, als `Person` behandeln und pseudonymisieren. Der teurere
Fehler ist das Leaken eines echten Namens.

Do:

```rust
let empfaenger = match gläubiger_id {
    Some(id) => Empfänger::Firma(id),
    None => Empfänger::Person(pseudonym), // unsicher → Person
};
```

Don't:

```rust
let empfaenger = Empfänger::Firma(name_als_klartext); // Default Firma leakt echte Namen
```

## Jeder Port sein schmales DTO — Antwort auf den Fingerabdruck

Kein geteiltes Gott-DTO. Der `Kategorisierer` antwortet auf den **Fingerabdruck, nie auf den
Text**, sodass Re-Identifikation strukturell unmöglich ist.

Do:

```rust
KategorieAntwort { fingerabdruck: Fingerabdruck, kategorie: Kategorie } // Schlüssel = Fingerabdruck
```

Don't:

```rust
KategorieAntwort { verwendungszweck: String, kategorie: Kategorie } // Schlüssel = Text ⇒ re-identifizierbar
```

## Fingerabdruck ist eine reine Domänenfunktion

`Buchung → Fingerabdruck` (inkl. Zahlungsdienstleister-Erkennung + Merchant-Token) lebt in
der Domäne, nicht im Adapter oder `io`.

Do:

```rust
// auswerten_domain
pub fn fingerabdruck(b: &Buchung) -> Fingerabdruck { /* rein */ }
```

Don't:

```rust
// obfuskierung_adapter
fn fingerabdruck(b: &Buchung) -> Fingerabdruck { /* Domänenwissen im Adapter */ }
```

## Der Seed bleibt lokal

Pseudonym-Stabilität ruht auf einem config-injizierten **Seed**, nicht auf einem
Klartext↔Pseudonym-Store. Der Seed darf lokal persistieren, erreicht aber **nie** ein
Egress-DTO.

Do:

```rust
let token = obfuscator.token(&self.seed, klartext); // Seed bleibt im Prozess/lokalen Store
```

Don't:

```rust
KategorieAnfrage { seed: self.seed.clone(), /* … */ } // Seed verlässt die Maschine — Leak
```
