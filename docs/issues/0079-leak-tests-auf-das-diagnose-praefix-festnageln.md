---
id: "0079"
title: Leak-Tests auf das Diagnose-Praefix festnageln
status: open
---

# Leak-Tests auf das Diagnose-Praefix festnageln

Härtet das Regressionsnetz hinter der Privacy-Invariante aus
[ADR-0015](../adr/0015-generischer-slice-store-ueber-storespec.md) (Punkt 5). **AFK.**

## What to build

Die Leak-Tests der io-Stores von `einlesen` und `obfuskierung` sichern heute **nur negativ**
zu: die Fehlermeldung enthält den bekannten Roh-Wert der Fixture nicht. Damit hängt der Test
allein an der Abwesenheit einer bestimmten Zeichenfolge — fiele die durchgereichte Diagnose
ganz weg oder käme sie aus einer anderen Quelle, bliebe er grün. Genau das ist aber der
Regress, den [0075](0075-obfuskierung-migrieren-und-storeunavailable-vereinheitlichen.md)
verhindert hat, und dessen drittes Akzeptanzkriterium ruht auf diesem Test.

Der Slice ergänzt die Zusicherung um den **positiven** Teil: die Meldung trägt das erwartete
Store-Präfix, *und* sie enthält keinen Rohwert. Damit fällt der Test sowohl, wenn ein
Zellwert durchschlägt, als auch, wenn die Diagnose verschwindet. Beide Stores werden gleich
gezogen, weil `obfuskierung` die Form von `einlesen` übernommen hat und eine einseitige
Härtung die nächste Drift wäre.

## Acceptance criteria

- [x] Der `obfuskierung`-Leak-Test sichert **positiv** das erwartete Store-Präfix der
      Diagnose **und** negativ die Abwesenheit des Rohwerts zu.
- [x] Der `einlesen`-Leak-Test, von dem die Form stammt, ist gleich nachgezogen.
- [x] Entfernt man die durchgereichte Diagnose probeweise aus dem jeweiligen Store, werden
      **beide** Tests rot (Mutationsprobe im Ticket dokumentiert, nicht eingecheckt).
- [x] Alle sechs Gates grün (`/cargo-six-gates`).

## Umsetzung

- Jeder der beiden Stores hält sein Diagnose-Präfix als **eine** Konstante
  (`UNAVAILABLE_PREFIX`: `einlesen store unavailable` / `obfuskiert store unavailable`), die
  `store_unavailable` setzt und der Test liest — kein zweites String-Literal, das wegdriften
  könnte.
- Die beidseitige Zusicherung liegt als `assert_redacted_diagnosis(outcomes, prefix, raw_value)`
  **einmal** in `pipeline_test_support` statt zweimal als `matches!`-Guard: genau ein
  `Warning`-Ausgang, dessen Grund das Präfix trägt (positiv) und den Rohwert nicht (negativ).
- Der `einlesen`-Test prüfte vorher `reason.contains("store")` — ein Präfix-Surrogat, das die
  Mutation unten **überlebt** hätte; er zieht jetzt dieselbe Konstante wie der Store.

## Mutationsprobe (nicht eingecheckt)

In **beiden** Stores die durchgereichte Diagnose am redigierenden Konstruktor vorbeigeführt,
sodass das Store-Präfix aus der Meldung fällt:

```rust
// crates/io/src/{einlesen,obfuskierung}/store.rs — impl StoreSpec
fn unavailable(error: rusqlite::Error) -> StoreUnavailable {
    StoreUnavailable { message: error.to_string() } // statt store_unavailable(&error.to_string())
}
```

`cargo test -p io --lib ein_unerreichbarer_store` → **beide** Tests rot (der dritte,
`auswerten`, bleibt unberührt grün):

```text
test obfuskierung::store::tests::ein_unerreichbarer_store_failt_terminal_und_redigiert_ohne_zu_publizieren ... FAILED
test einlesen::store::tests::ein_unerreichbarer_store_failt_terminal_und_redigiert_ohne_zu_publizieren ... FAILED

die Diagnose muss das Präfix `obfuskiert store unavailable` tragen (sonst ist sie weg oder
stammt aus anderer Quelle): could not store obfuscated booking: no such table: obfuskiert
die Diagnose muss das Präfix `einlesen store unavailable` tragen (sonst ist sie weg oder
stammt aus anderer Quelle): store failed: no such table: einlesen_lines
```

Beide Meldungen zeigen zugleich, warum die alten Zusicherungen den Regress nicht gefangen
hätten: der Grund enthält weiterhin keine Roh-IBAN (die negative Prüfung wäre grün geblieben),
und `store failed: …` enthält sogar das Wort `store` (das alte `einlesen`-Surrogat wäre grün
geblieben). Mutation danach zurückgenommen, alle sechs Gates grün.

## Blocked by

None - can start immediately.
