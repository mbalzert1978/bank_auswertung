use crate::abstractions::{CurrencySeam, ResolvedCurrency};

/// Deterministischer Fake des Währungs-Seams (Vorbild [`FakeCalendarSeam`](super::FakeCalendarSeam)):
/// löst die wenigen Test-Währungen fest verdrahtet auf — kanonischer Code + ISO-4217-Minor-Unit
/// (EUR → 2, JPY → 0, BHD → 3) — genug, um die währungs-bewusste Skalierung end-to-end zu
/// spezifizieren. **Case-insensitiv** wie die echte Auflösung (`eur` == `EUR`). Die **echte**
/// ISO-Quelle ist eine battle-tested Lib in `io` hinter dem realen Seam; dieser Fake erhebt keinen
/// Vollständigkeits-Anspruch. Ein unbekannter Code ergibt `None` (die Domäne mappt das terminal auf
/// `CurrencyUnknown`).
pub(crate) struct FakeCurrencySeam;

impl CurrencySeam for FakeCurrencySeam {
    fn resolve(&self, code: &str) -> Option<ResolvedCurrency> {
        let (canonical, minor_unit) = match code.to_ascii_uppercase().as_str() {
            "EUR" => ("EUR", 2),
            "JPY" => ("JPY", 0),
            "BHD" => ("BHD", 3),
            _ => return None,
        };
        Some(ResolvedCurrency {
            code: canonical.to_string(),
            minor_unit,
        })
    }
}
