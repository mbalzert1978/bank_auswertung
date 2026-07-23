//! Die echte io-Erfüllung des öffentlichen [`CurrencySeam`] des `einlesen`-Adapters
//! ([Issue 0055](../../../../docs/issues/0055-waehrungs-bewusster-betrags-decode-minor-unit-per-iso-4217-von-der-domaene-hoch.md)):
//! ein **Adapter**, der die battle-tested [`rusty_money`]-Lib (ISO 4217, auf `rust_decimal` gebaut)
//! auf unsere Seam-Signatur andockt — rein, kein I/O. Sie löst einen Währungscode zu Identität **und**
//! Minor Unit auf, **ohne** handgepflegte Tabelle; die Domäne bleibt so Wahrheit für die Währung, ohne
//! die Nachkommastellen selbst zu führen.

use einlesen_adapter::{CurrencySeam, ResolvedCurrency};
use rusty_money::iso;

/// Der echte Währungs-Seam: schlägt Code + Minor Unit über [`rusty_money`] (ISO 4217) nach.
/// **Zustandslos** — ein Wert genügt (EUR → 2, JPY → 0, BHD → 3). Der Lookup ist **case-insensitiv**
/// (der rohe CSV-Wert kann `eur` tragen); der zurückgegebene Code ist der kanonische Großbuchstaben-Code.
pub struct IsoCurrencies;

impl CurrencySeam for IsoCurrencies {
    fn resolve(&self, code: &str) -> Option<ResolvedCurrency> {
        iso::find(&code.to_ascii_uppercase()).map(|currency| ResolvedCurrency {
            code: currency.iso_alpha_code.to_string(),
            minor_unit: currency.exponent,
        })
    }
}
