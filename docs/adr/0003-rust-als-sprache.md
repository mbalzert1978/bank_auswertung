# Rust als Implementierungssprache

Wir implementieren in Rust statt Python oder C#. Das Datenschutzmodell ruht auf
der Obfuskierung — in Rust lässt sich die **strukturelle** Grenze compilergeprüft
erzwingen: Die Rohdaten-Newtypes (`Iban`, `Bic`) leben nur im eingehenden
Adapter, leiten kein `Serialize` ab und landen in keinem DTO; die Domäne hält
ausschließlich obfuskierte Value Objects. Der einzige serialisierbare Ausgang ist
der schmale `KategorieAnfrage`-DTO des `Kategorisierer`-Ports — er _kann_ keinen
strukturellen Identifikator (IBAN/BIC/Kontonr.) tragen. Ein solcher Leak wird zum
Compile-Fehler statt zur Review-Hoffnung.

Was der Compiler **nicht** beweisen kann: dass im Empfänger-/Verwendungszweck-
Freitext kein Identifikator eingebettet ist (eine IBAN im Verwendungszweck, ein
Klarname im Betreff). Dieser Freitext ist deshalb kein roher `String`, sondern
ein eigenes Value Object `ObfuskierterText`, dessen Konstruktor best-effort
scrubbt (IBAN/BIC-Muster, bekannte Personen-Token), fail-safe. Der Typ erzwingt
**einen** Engpass, durch den aller Freitext muss — die Vollständigkeit des
Scrubbings bleibt heuristisch, nicht bewiesen. Das ist die ehrliche Grenze des
Modells.

Dazu Summentypen (`enum` + erschöpfendes `match`) für „illegale Zustände
unmöglich" (ausgeführt in
[ADR-0005](0005-domaenenmodell-value-objects-und-summentypen.md)) und ein
einziges lokales Binary ohne Runtime (Datenschutz-Bonus).

Bewusst in Kauf genommen: langsamerer Start, mehr Zeremonie, LLM-Anbindung nur
per roher REST statt fertigem SDK.

## Consequences

- `pyproject.toml` und `.python-version` entfallen; das Projekt wird ein
  Cargo-Workspace.
- **Der hexagonale Schnitt wird auf Crate-Ebene erzwungen:** Die Domänen-Crate
  hängt von keiner I/O-Crate ab (kein `reqwest`, kein `rusqlite`) und _kann_ daher
  kein I/O. Adapter leben in eigenen Crates.
- **`serde` ist erlaubt — aber nur auf obfuskierten DTOs.** Die Domänen-Crate darf
  `serde` für ihren Ausgangs-DTO (`KategorieAnfrage`) führen; die Leak-Garantie
  ruht **nicht** auf der Abwesenheit von `serde`, sondern darauf, dass die
  Rohdaten-Newtypes (`Iban`, `Bic`) kein `Serialize` ableiten und deshalb in
  keinen serialisierbaren DTO einbettbar sind — ein Versuch ist ein
  Compile-Fehler, keine Review-Hoffnung.
