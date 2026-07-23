use crate::value_objects::{Currency, NonEmptyString};

/// Der von der Domäne **verlangte** Port, der einen rohen Währungscode zu einer [`Currency`] deutet —
/// **Identität und Minor Unit** (ISO 4217) in einem. Die Domäne bleibt dependency-frei und führt
/// **keine** Nachkommastellen-Tabelle: welche Codes gültig sind und wie viele Nachkommastellen jeder
/// hat, weiß die battle-tested ISO-Quelle hinter diesem Port in `io`; die Tests unterschieben einen
/// Fake. Der Vertrag ist wert-typisiert: die Domäne fordert einen `NonEmptyString` an, **kein**
/// primitives `&str` — der Adapter mappt den rohen CSV-Wert. Der Port trägt **nur** die eine
/// Fehlbedeutung „unbekannte Währung" als `None` (nicht das ganze Slice-Fehler-Enum); das Aggregat
/// mappt `None` an einem Ort auf den terminalen `CurrencyUnknown`. Vorbild: [`Calendar`](crate::Calendar)
/// deutet so das Datum, hinter dem in `io` `chrono` steckt.
pub trait Currencies {
    /// Deutet den rohen Währungscode zu einer [`Currency`], oder `None`, wenn es keine der Lib
    /// bekannte Währung ist.
    fn resolve(&self, raw: &NonEmptyString) -> Option<Currency>;
}
