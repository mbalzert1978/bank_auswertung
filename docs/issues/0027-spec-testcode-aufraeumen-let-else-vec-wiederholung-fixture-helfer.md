---
id: "0027"
title: Spec-Testcode aufräumen (let-else, vec-Wiederholung, Fixture-Helfer)
status: closed
---

# Spec-Testcode aufräumen (let-else, vec-Wiederholung, Fixture-Helfer)

**Typ:** AFK

> Idiomatik-Review-Befund (Schwere: Niedrig/Mittel im Testcode). Beide Spec-Crates sind
> sauber, tragen aber ein paar nicht-idiomatische Stellen. Die Black-Box-Konvention
> (pure Tests über die Fluent-TestApi, Ganze-DTO-Vergleich) bleibt unangetastet — nur
> die Formulierung wird idiomatischer.

## Was zu bauen ist

Idiomatik-Cleanup des Testcodes in `einlesen_specs` und `obfuskierung_specs`, ohne die
geprüften Verhalten oder Assertion-Semantik zu ändern:

- **`obfuskierung_specs` — Doppel-Match auflösen:** ein Test prüft dieselbe Enum-Variante
  erst per `assert!(matches!(...))` und extrahiert sie dann per `if let` (der still
  ausfallen kann). In Edition 2024 zu `let … else { panic!(...) }` zusammenführen — ein
  Match statt zwei, keine klammheimliche Extraktion. Analog die redundante
  Varianten-plus-Wortlaut-Prüfung straffen.
- **`einlesen_specs` — Wiederholungs-Literale:** `vec!["Ingested", "Ingested", "Ingested"]`
  bzw. dreifach identische IBAN-Literale werden `vec![x; 3]` (die Aussage „drei gleiche"
  statt Abzählen); die drei nur im belanglosen Namen abweichenden Records lassen sich als
  `fold` über die Namen ausdrücken (der repo-geforderte deklarativ-funktionale Stil).
- **Fixture-Helfer / Konsistenz:** die in `obfuskierung_specs` mehrfach wörtlich
  wiederholte Arrange-Kette in einen spec-lokalen Helfer ziehen; die `.as_str()`-Zeremonie
  beidseitig im `assert_eq!` entfernen, wo der Feldtyp `PartialEq` hat; den
  `assert_eq!(ergebnis, Ok(()))`-Ausreißer in `einlesen_specs` an den sonstigen
  `.expect(...)`-Stil angleichen.

## Abnahmekriterien

- [ ] Der Doppel-Match in `obfuskierung_specs` ist durch `let … else` ersetzt; die
      redundante Varianten-/Wortlaut-Prüfung ist gestrafft.
- [ ] Wiederholungs-Literale in `einlesen_specs` nutzen `vec![x; n]`; die drei
      Namens-Varianten sind als `fold` (oder gleichwertig deklarativ) ausgedrückt.
- [ ] Die wiederholte Arrange-Kette in `obfuskierung_specs` ist in einen spec-lokalen
      Fixture-Helfer gezogen; unnötige `.as_str()`-Zeremonie entfernt; der
      `assert_eq!(Ok(()))`-Ausreißer an den `.expect(...)`-Stil angeglichen.
- [ ] Die Specs bleiben pure Black-Box über die Fluent-TestApi (Ganze-DTO-Vergleich
      unverändert); dieselben Verhalten werden geprüft; kein Slice-Produktivcode berührt.
- [ ] `cargo test` grün (beide Spec-Crates); `cargo clippy --all-targets`/`fmt --check`
      und `cargo test -p architecture` grün.

## Blockiert durch

- None - can start immediately.

**Parallel-sicher:** berührt ausschließlich die `*_specs`-Crates (kein Slice-Produktivcode),
disjunkt von 0023–0026. Kann jederzeit parallel zu jedem anderen Issue laufen.
