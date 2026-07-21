//! Die echte io-Erfüllung des öffentlichen [`MoneySeam`] des `einlesen`-Adapters
//! ([Issue 0050](../../../../docs/issues/0050-money-calendar-decode-seams-real.md)): ein **Adapter**,
//! der die battle-tested [`rust_decimal`]-Lib auf unsere Seam-Signatur andockt — rein, kein I/O. Sie
//! tritt im End-to-End-Test an die Stelle des In-Memory-Fakes.
//!
//! Der Seam ist **bewusst währungs-agnostisch** bei fester Skala 2 (Cent); die währungs-bewusste
//! Minor-Unit-Deutung per ISO 4217 ist ausgelagert
//! ([Issue 0055](../../../../docs/issues/0055-waehrungs-bewusster-betrags-decode-minor-unit-per-iso-4217-von-der-domaene-hoch.md)).
//! `rust_decimal` rechnet exakt (kein `f64`-Rundungsfehler); unsere **dünne** Nutzungslogik streift
//! das führende Vorzeichen ab und bringt die deutsche Gruppierung/Dezimalstelle in die Form, die die
//! Lib parst. Ein undeutbarer Wert ergibt `None`, was das Aggregat terminal auf `AmountInvalid` mappt.

use core::str::FromStr;

use einlesen_adapter::{DecodedAmount, MoneySeam, Sign};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Der echte Geld-Seam: dockt `rust_decimal` auf [`MoneySeam`] an. **Zustandslos** — ein Wert genügt.
pub struct GermanMoney;

impl MoneySeam for GermanMoney {
    fn decode(&self, raw: &str) -> Option<DecodedAmount> {
        // Das führende Vorzeichen ist **Pflicht** (Seam-Vertrag: ohne deutbares Vorzeichen `None`,
        // nie still-positiv) und trennt den Ziffernkörper ab.
        let (sign, body) = raw
            .strip_prefix('-')
            .map(|rest| (Sign::Negative, rest))
            .or_else(|| raw.strip_prefix('+').map(|rest| (Sign::Positive, rest)))?;

        // Deutsche Darstellung → das, was `rust_decimal` parst: Tausenderpunkte sind reine Gruppierung
        // (entfallen), das Dezimalkomma wird zum Punkt. Der Vorzeichen-Teil ist schon abgestreift.
        let normalised = body.replace('.', "").replace(',', ".");
        let euros = Decimal::from_str(&normalised).ok()?;

        // Feste Skala 2: der Betrag muss in ganzen Cent aufgehen (drei+ Nachkommastellen sind für
        // eine Cent-Währung kein gültiger Betrag). `rust_decimal` rechnet das exakt.
        let cents = euros.checked_mul(Decimal::ONE_HUNDRED)?;
        if !cents.fract().is_zero() {
            return None;
        }

        Some(DecodedAmount {
            cent: cents.to_u64()?,
            sign,
        })
    }
}
