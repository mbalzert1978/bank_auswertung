use crate::abstractions::{DecodedAmount, MoneySeam, Sign};

/// Deterministischer Fake **ohne Format-Anspruch** (Vorbild [`FakeCalendarSeam`](super::FakeCalendarSeam)):
/// streift das Vorzeichen ab und parst die Restziffern naiv (Tausenderpunkt entfällt,
/// Komma trennt die Nachkommastellen) — genug für die Fixtures. Die echten deutschen
/// Formatregeln (Gruppierung, Stellenzahl) sind io-Aufgabe hinter dem echten Seam
/// (deferred), nicht hier. Ohne deutbares Vorzeichen oder bei Nicht-Ziffern ergibt sich
/// `None` (die Domäne sieht dann `AmountInvalid`).
pub(crate) struct FakeMoneySeam;

impl MoneySeam for FakeMoneySeam {
    fn decode(&self, raw: &str) -> Option<DecodedAmount> {
        let (sign, digits) = raw
            .strip_prefix('-')
            .map(|rest| (Sign::Negative, rest))
            .or_else(|| raw.strip_prefix('+').map(|rest| (Sign::Positive, rest)))?;
        let (euro, cent) = digits.split_once(',').unwrap_or((digits, ""));
        let euro: u64 = euro.replace('.', "").parse().ok()?;
        let cent: u64 = if cent.is_empty() {
            0
        } else {
            cent.parse().ok()?
        };
        Some(DecodedAmount {
            cent: euro.checked_mul(100)?.checked_add(cent)?,
            sign,
        })
    }
}
