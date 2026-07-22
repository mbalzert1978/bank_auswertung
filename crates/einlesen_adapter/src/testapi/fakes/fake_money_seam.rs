use crate::abstractions::{DecodedAmount, MoneySeam, Sign};

/// Deterministischer Fake **ohne Format-Anspruch** (Vorbild [`FakeCalendarSeam`](super::FakeCalendarSeam)):
/// streift das Vorzeichen ab und zerlegt die Restziffern naiv (Tausenderpunkt entfällt, Komma
/// trennt die Nachkommastellen) in die **unskalierten** Bestandteile (`units`, `decimal_places`) —
/// genug für die Fixtures. Die währungs-bewusste Skalierung auf die Minor Units macht die
/// `MoneyBridge` über den `CurrencySeam`, **nicht** dieser Fake; die echten deutschen Formatregeln
/// (Gruppierung, Stellenzahl) sind io-Aufgabe hinter dem echten Seam. Ohne deutbares Vorzeichen oder
/// bei Nicht-Ziffern ergibt sich `None` (die Domäne sieht dann `AmountInvalid`).
pub(crate) struct FakeMoneySeam;

impl MoneySeam for FakeMoneySeam {
    fn decode(&self, raw: &str) -> Option<DecodedAmount> {
        let (sign, digits) = raw
            .strip_prefix('-')
            .map(|rest| (Sign::Negative, rest))
            .or_else(|| raw.strip_prefix('+').map(|rest| (Sign::Positive, rest)))?;
        let (integer_part, fraction) = digits.split_once(',').unwrap_or((digits, ""));
        let decimal_places = u32::try_from(fraction.len()).ok()?;
        // Ziffernkörper = Ganzzahl-Teil (Tausenderpunkt entfernt) gefolgt vom Nachkomma-Teil; der
        // Wert ist `units / 10^decimal_places` — die Bridge skaliert ihn auf die Minor Units.
        let units: u64 = format!("{}{fraction}", integer_part.replace('.', ""))
            .parse()
            .ok()?;
        Some(DecodedAmount {
            units,
            decimal_places,
            sign,
        })
    }
}
